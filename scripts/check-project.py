"""Read-only project integrity preflight. Python standard library only."""
from pathlib import Path
import hashlib
import re
import struct
import sys
import zlib

ROOT = Path(__file__).resolve().parent.parent
errors = []
def require(condition, message):
    if not condition:
        errors.append(message)

def rtss_opt_out(path):
    """Check the real PE export and its DWORD, not just a string in the binary."""
    data = path.read_bytes()
    try:
        pe = struct.unpack_from('<I', data, 0x3c)[0]
        require(data[:2] == b'MZ' and data[pe:pe+4] == b'PE\0\0', f'Invalid PE: {path}')
        count = struct.unpack_from('<H', data, pe+6)[0]
        opt = pe+24
        opt_size = struct.unpack_from('<H', data, pe+20)[0]
        magic = struct.unpack_from('<H', data, opt)[0]
        sections = [struct.unpack_from('<IIII', data, opt+opt_size+i*40+8) for i in range(count)]
        def raw(rva):
            for virtual_size, address, raw_size, offset in sections:
                if address <= rva < address+max(virtual_size, raw_size):
                    return offset+rva-address
            raise ValueError('Unmapped RVA')
        directory = opt+(112 if magic == 0x20b else 96)
        exports = raw(struct.unpack_from('<I', data, directory)[0])
        names_count = struct.unpack_from('<I', data, exports+24)[0]
        functions, names, ordinals = struct.unpack_from('<III', data, exports+28)
        for index in range(names_count):
            offset = raw(struct.unpack_from('<I', data, raw(names)+index*4)[0])
            name = data[offset:data.index(0, offset)]
            if name == b'RTSSHooksCompatibility':
                ordinal = struct.unpack_from('<H', data, raw(ordinals)+index*2)[0]
                value = raw(struct.unpack_from('<I', data, raw(functions)+ordinal*4)[0])
                require(struct.unpack_from('<I', data, value)[0] == 0, f'Invalid RTSS opt-out DWORD: {path}')
                return
        errors.append(f'Missing RTSS opt-out export: {path}')
    except (struct.error, ValueError, IndexError):
        errors.append(f'Cannot verify RTSS PE export: {path}')

def png(path):
    data = path.read_bytes()
    require(data[:8] == b"\x89PNG\r\n\x1a\n", f"Invalid PNG signature: {path}")
    offset = 8
    try:
        while offset < len(data):
            length = struct.unpack(">I", data[offset:offset+4])[0]
            end = offset + length + 12
            require(end <= len(data), f"Truncated PNG: {path}")
            if end > len(data):
                return
            expected = struct.unpack(">I", data[end-4:end])[0]
            require(zlib.crc32(data[offset+4:end-4]) == expected, f"PNG CRC mismatch: {path}")
            if data[offset+4:offset+8] == b"IEND":
                require(end == len(data), f"Trailing PNG data: {path}")
                return
            offset = end
        errors.append(f"Missing PNG IEND: {path}")
    except struct.error:
        errors.append(f"Invalid PNG chunks: {path}")

version = re.search(r'^version = "([^"]+)"', (ROOT/'Cargo.toml').read_text(), re.M)[1]
lock = (ROOT/'Cargo.lock').read_text()
require(f'name = "qbit-remote"\nversion = "{version}"' in lock, 'Cargo.lock version mismatch')
for file in ['README.md', 'Release/README.md', 'CHANGELOG.md']:
    text = (ROOT/file).read_text(encoding='utf-8')
    require(version in text, f'Current version absent: {file}')
for file in list((ROOT/'src').rglob('*.rs')) + [ROOT/'build.rs']:
    for target in re.findall(r'include_(?:bytes|str)!\("([^"\n]+)"\)', file.read_text(encoding='utf-8')):
        require((file.parent/target).is_file(), f'Missing embedded resource: {file}: {target}')
for target in ['assets/brand-lockup-source.png','assets/brand-forest.png','assets/logo-master.png',
               '.github/workflows/windows.yml','.github/workflows/macos.yml','scripts/build-macos.sh']:
    require((ROOT/target).is_file(), f'Missing required input: {target}')
for file in (ROOT/'assets').rglob('*.png'):
    png(file)
raster = ROOT/'assets/logo.rgba'
require(raster.is_file() and raster.stat().st_size == 256*256*4, 'Missing or invalid logo RGBA')
for file in [ROOT/'README.md', ROOT/'Release/README.md', *list((ROOT/'docs').glob('*.md'))]:
    for target in re.findall(r'\]\(([^)]+)\)', file.read_text(encoding='utf-8')):
        if '://' not in target and not target.startswith('#'):
            require((file.parent/target.split('#')[0]).exists(), f'Broken local link: {file}: {target}')
require(b'\r' not in (ROOT/'scripts/build-macos.sh').read_bytes(), 'macOS shell script must use LF')
if '--release' in sys.argv:
    paths = [ROOT/'dist/qbit-remote.exe',ROOT/'Release/qbit-remote.exe']
    require(all(p.is_file() for p in paths), 'Missing Windows release binary')
    if all(p.is_file() for p in paths):
        for path in paths:
            rtss_opt_out(path)
        require(len({hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}) == 1, 'Release SHA256 mismatch')
    require(len(list((ROOT/'dist').glob('*.exe'))) == 1, 'dist must contain one EXE')
    require({p.name for p in (ROOT/'Release').iterdir()} == {'qbit-remote.exe','README.md'}, 'Unexpected Release contents')
if errors:
    print('\n'.join(errors))
    sys.exit(1)
print(f'Project {version}: embedded resources, PNG CRC, source inputs, versions, links and line endings OK')

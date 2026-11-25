#import <AppKit/AppKit.h>
#import <objc/runtime.h>

extern void qbit_macos_open(const char *value, bool is_file);
extern void qbit_macos_reopen(void);

static __weak NSWindow *main_window;

static void open_urls(__unused id delegate, __unused SEL action,
                      __unused NSApplication *app, NSArray<NSURL *> *urls) {
    for (NSURL *url in urls) {
        bool is_file = url.isFileURL;
        qbit_macos_open(is_file ? url.fileSystemRepresentation : url.absoluteString.UTF8String,
                       is_file);
    }
}

static BOOL reopen(__unused id delegate, __unused SEL action,
                   __unused NSApplication *app, BOOL has_visible_windows) {
    if (!has_visible_windows) {
        qbit_macos_reopen();
    }
    return has_visible_windows;
}

@interface QBitRemoteApplication : NSApplication
@end

@implementation QBitRemoteApplication
- (void)setDelegate:(id<NSApplicationDelegate>)delegate {
    // Winit owns the delegate and its lifecycle. Add only the optional callbacks it leaves open.
    if (delegate) {
        Class cls = object_getClass(delegate);
        Protocol *protocol = @protocol(NSApplicationDelegate);
        SEL open = @selector(application:openURLs:);
        SEL show = @selector(applicationShouldHandleReopen:hasVisibleWindows:);
        class_addMethod(cls, open, (IMP)open_urls,
                        protocol_getMethodDescription(protocol, open, NO, YES).types);
        class_addMethod(cls, show, (IMP)reopen,
                        protocol_getMethodDescription(protocol, show, NO, YES).types);
    }
    [super setDelegate:delegate];
}
@end

void qbit_macos_install(void) {
    @autoreleasepool {
        [QBitRemoteApplication sharedApplication];
    }
}

void qbit_macos_set_window(void *view) {
    main_window = ((__bridge NSView *)view).window;
}

void qbit_macos_show_window(void) {
    [main_window deminiaturize:nil];
    [main_window makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
}

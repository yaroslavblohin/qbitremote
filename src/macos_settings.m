#import <AppKit/AppKit.h>
#import <CoreServices/CoreServices.h>
#import <ServiceManagement/ServiceManagement.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>

typedef void (*QBitResult)(void *, const char *);

enum QBitLoginStatus { LoginOff, LoginOn, LoginNeedsApproval, LoginMoved, LoginUnavailable };

static void report(QBitResult result, void *context, NSString *message) {
    result(context, message.UTF8String);
}

bool qbit_macos_has_app_bundle(void) {
    NSBundle *bundle = NSBundle.mainBundle;
    return bundle.bundleIdentifier.length > 0 &&
           [bundle.bundleURL.pathExtension.lowercaseString isEqualToString:@"app"];
}

#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
// macOS 11 has the Launch Services setters, while newer versions manage consent in AppKit.
static NSString *register_handler(NSBundle *bundle) {
    OSStatus status = LSRegisterURL((__bridge CFURLRef)bundle.bundleURL, true);
    return status == noErr ? nil : [NSString stringWithFormat:@"Launch Services: %d", (int)status];
}

static void set_handler(NSBundle *bundle, NSString *identifier, bool scheme,
                        void (^completion)(NSString *)) {
    NSWorkspace *workspace = NSWorkspace.sharedWorkspace;
    UTType *type = scheme ? nil : [UTType typeWithIdentifier:identifier];
    if (!scheme && type == nil) {
        completion(@"macOS не зарегистрировала тип файла.");
        return;
    }
    NSURL *current = nil;
    if (scheme) {
        current = [workspace URLForApplicationToOpenURL:[NSURL URLWithString:
                       [identifier stringByAppendingString:@":x"]]];
    } else if (@available(macOS 12.0, *)) {
        current = [workspace URLForApplicationToOpenContentType:type];
    } else {
        current = CFBridgingRelease(LSCopyDefaultApplicationURLForContentType(
            (__bridge CFStringRef)identifier, kLSRolesAll, NULL));
    }
    if (current && [current.path.stringByResolvingSymlinksInPath
                   isEqualToString:bundle.bundlePath.stringByResolvingSymlinksInPath]) {
        completion(nil);
        return;
    }
    if (@available(macOS 12.0, *)) {
        void (^finished)(NSError *) = ^(NSError *error) {
            dispatch_async(dispatch_get_main_queue(), ^{ completion(error.localizedDescription); });
        };
        if (scheme) {
            [workspace setDefaultApplicationAtURL:bundle.bundleURL toOpenURLsWithScheme:identifier
                               completionHandler:finished];
        } else {
            [workspace setDefaultApplicationAtURL:bundle.bundleURL toOpenContentType:type
                               completionHandler:finished];
        }
    } else {
        CFStringRef bundleID = (__bridge CFStringRef)bundle.bundleIdentifier;
        CFStringRef handler = (__bridge CFStringRef)identifier;
        OSStatus status = scheme
            ? LSSetDefaultHandlerForURLScheme(handler, bundleID)
            : LSSetDefaultRoleHandlerForContentType(handler, kLSRolesAll, bundleID);
        completion(status == noErr ? nil : [NSString stringWithFormat:@"Launch Services: %d", (int)status]);
    }
}
#pragma clang diagnostic pop

static void apply_handlers(NSBundle *app, NSBundle *documents, NSArray<NSString *> *identifiers,
                           NSArray<NSString *> *labels, NSUInteger index,
                           NSMutableArray<NSString *> *errors, void *context, QBitResult result) {
    if (index == identifiers.count) {
        report(result, context, errors.count ? [errors componentsJoinedByString:@"\n"] : nil);
        return;
    }
    bool scheme = index == identifiers.count - 1;
    set_handler(scheme ? app : documents, identifiers[index], scheme, ^(NSString *error) {
        if (error) {
            [errors addObject:[NSString stringWithFormat:@"%@ — %@", labels[index], error]];
        }
        apply_handlers(app, documents, identifiers, labels, index + 1, errors, context, result);
    });
}

void qbit_macos_apply_associations(unsigned int resource, void *context, QBitResult result) {
    NSBundle *app = NSBundle.mainBundle;
    if (!qbit_macos_has_app_bundle()) {
        report(result, context, @"Откройте qBit Remote.app.");
        return;
    }
    NSBundle *documents = app;
    if (resource != 2) {
        NSString *name = [NSString stringWithFormat:@"Contents/Helpers/qBit Remote %u.app", resource];
        documents = [NSBundle bundleWithURL:[app.bundleURL URLByAppendingPathComponent:name]];
        if (documents.bundleIdentifier.length == 0) {
            report(result, context, @"В пакете qBit Remote отсутствует выбранный значок.");
            return;
        }
    }

    NSArray<NSDictionary *> *types = [app objectForInfoDictionaryKey:@"CFBundleDocumentTypes"];
    NSArray<NSDictionary *> *schemes = [app objectForInfoDictionaryKey:@"CFBundleURLTypes"];
    NSString *torrent = [types.firstObject[@"LSItemContentTypes"] firstObject];
    NSString *magnet = [types.lastObject[@"LSItemContentTypes"] firstObject];
    NSString *scheme = [schemes.firstObject[@"CFBundleURLSchemes"] firstObject];
    if (types.count != 2 || !torrent.length || !magnet.length || !scheme.length) {
        report(result, context, @"В пакете приложения отсутствуют ассоциации.");
        return;
    }

    // Finder reads the document icon from the default handler. Each alternate icon has
    // a signed handler inside the app, so switching never modifies the installed bundle.
    NSString *error = register_handler(app);
    if (!error && documents != app) {
        error = register_handler(documents);
    }
    if (error) {
        report(result, context, error);
        return;
    }
    apply_handlers(app, documents, @[torrent, magnet, scheme], @[@".torrent", @".magnet", @"magnet:"],
                   0, [NSMutableArray array], context, result);
}

static NSURL *legacy_login_url(void) {
    NSURL *library = [NSFileManager.defaultManager URLsForDirectory:NSLibraryDirectory
                                                          inDomains:NSUserDomainMask].firstObject;
    NSURL *agents = [library URLByAppendingPathComponent:@"LaunchAgents" isDirectory:YES];
    NSString *name = [NSBundle.mainBundle.bundleIdentifier stringByAppendingString:@".login.plist"];
    return [agents URLByAppendingPathComponent:name isDirectory:NO];
}

static NSArray<NSString *> *legacy_arguments(NSURL *bundle) {
    return @[@"/usr/bin/open", @"-a", bundle.path];
}

static bool write_legacy_login_agent(NSURL *url, NSURL *bundle, NSString *identifier, NSError **error) {
    if (![NSFileManager.defaultManager createDirectoryAtURL:url.URLByDeletingLastPathComponent
                                withIntermediateDirectories:YES attributes:nil error:error]) {
        return false;
    }
    NSDictionary *job = @{
        @"Label": [identifier stringByAppendingString:@".login"],
        @"ProgramArguments": legacy_arguments(bundle),
        @"LimitLoadToSessionType": @"Aqua",
        @"RunAtLoad": @YES,
    };
    NSData *plist = [NSPropertyListSerialization dataWithPropertyList:job
                         format:NSPropertyListXMLFormat_v1_0 options:0 error:error];
    return [plist writeToURL:url options:NSDataWritingAtomic error:error];
}

static bool remove_legacy_login_agent(NSURL *url, NSError **error) {
    if (![NSFileManager.defaultManager fileExistsAtPath:url.path]) {
        return true;
    }
    return [NSFileManager.defaultManager removeItemAtURL:url error:error];
}

static int legacy_login_status(NSURL *url, NSURL *bundle) {
    if (![NSFileManager.defaultManager fileExistsAtPath:url.path]) {
        return LoginOff;
    }
    NSDictionary *job = [NSDictionary dictionaryWithContentsOfURL:url error:NULL];
    return [legacy_arguments(bundle) isEqual:job[@"ProgramArguments"]] ? LoginOn : LoginMoved;
}

int qbit_macos_login_status(void) {
    if (!qbit_macos_has_app_bundle()) {
        return LoginUnavailable;
    }
    if (@available(macOS 13.0, *)) {
        SMAppServiceStatus status = SMAppService.mainAppService.status;
        if (status == SMAppServiceStatusEnabled) {
            return LoginOn;
        }
        if (status == SMAppServiceStatusRequiresApproval) {
            return LoginNeedsApproval;
        }
    }
    return legacy_login_status(legacy_login_url(), NSBundle.mainBundle.bundleURL);
}

void qbit_macos_set_login(bool enable, void *context, QBitResult result) {
    if (!qbit_macos_has_app_bundle()) {
        report(result, context, @"Откройте qBit Remote.app.");
        return;
    }
    NSURL *url = legacy_login_url();
    NSError *error = nil;
    bool updated = true;
    if (@available(macOS 13.0, *)) {
        SMAppService *service = SMAppService.mainAppService;
        SMAppServiceStatus status = service.status;
        bool registered = status == SMAppServiceStatusEnabled || status == SMAppServiceStatusRequiresApproval;
        if (enable && !registered) {
            updated = [service registerAndReturnError:&error];
        } else if (!enable && registered) {
            updated = [service unregisterAndReturnError:&error];
        }
        if (updated) {
            updated = remove_legacy_login_agent(url, &error);
        }
    } else if (enable) {
        NSBundle *bundle = NSBundle.mainBundle;
        updated = write_legacy_login_agent(url, bundle.bundleURL, bundle.bundleIdentifier, &error);
    } else {
        updated = remove_legacy_login_agent(url, &error);
    }
    report(result, context, updated ? nil : (error.localizedDescription ?: @"Не удалось обновить автозапуск."));
}

void qbit_macos_open_login_settings(void) {
    if (@available(macOS 13.0, *)) {
        [SMAppService openSystemSettingsLoginItems];
    }
}

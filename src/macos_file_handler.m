#import <AppKit/AppKit.h>

// Each bundle needs its own executable UUID even though the handlers run the same code.
__attribute__((used)) static const unsigned int file_icon_resource = QBIT_FILE_ICON_RESOURCE;

@interface QBitFileHandler : NSObject <NSApplicationDelegate>
@property NSUInteger pending;
@end

@implementation QBitFileHandler
- (void)application:(NSApplication *)application openURLs:(NSArray<NSURL *> *)urls {
    // Contents/Helpers/<handler>.app lives inside the qBit Remote bundle it opens.
    NSURL *app = NSBundle.mainBundle.bundleURL.URLByDeletingLastPathComponent
        .URLByDeletingLastPathComponent.URLByDeletingLastPathComponent;
    self.pending++;
    [NSWorkspace.sharedWorkspace openURLs:urls
                     withApplicationAtURL:app
                            configuration:NSWorkspaceOpenConfiguration.configuration
                        completionHandler:^(__unused NSRunningApplication *running, NSError *error) {
        dispatch_async(dispatch_get_main_queue(), ^{
            if (error) {
                NSAlert *alert = [NSAlert new];
                alert.messageText = @"Не удалось открыть файл в qBit Remote";
                alert.informativeText = error.localizedDescription;
                [alert runModal];
            }
            if (--self.pending == 0) {
                [application terminate:nil];
            }
        });
    }];
}
@end

int main(void) {
    @autoreleasepool {
        NSApplication *application = NSApplication.sharedApplication;
        __attribute__((objc_precise_lifetime)) QBitFileHandler *delegate = [QBitFileHandler new];
        application.delegate = delegate;
        [application run];
    }
}

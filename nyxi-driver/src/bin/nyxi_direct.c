#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <IOKit/IOKitLib.h>
#include <IOKit/hid/IOHIDManager.h>
#include <IOKit/hid/IOHIDKeys.h>

int main(void) {
    printf("========================================================\n");
    printf("     NATIVE macOS IOKit DIRECT HID DIAGNOSTIC & READ   \n");
    printf("========================================================\n");

    IOHIDManagerRef manager = IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone);
    if (!manager) {
        printf("Failed to create IOHIDManager\n");
        return 1;
    }

    // Match vendor 0x3151, product 0x4015
    CFMutableDictionaryRef matchDict = CFDictionaryCreateMutable(kCFAllocatorDefault, 0,
        &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks);
    int vid = 0x3151;
    int pid = 0x4015;
    CFNumberRef vidNum = CFNumberCreate(kCFAllocatorDefault, kCFNumberIntType, &vid);
    CFNumberRef pidNum = CFNumberCreate(kCFAllocatorDefault, kCFNumberIntType, &pid);
    CFDictionarySetValue(matchDict, CFSTR(kIOHIDVendorIDKey), vidNum);
    CFDictionarySetValue(matchDict, CFSTR(kIOHIDProductIDKey), pidNum);
    CFRelease(vidNum);
    CFRelease(pidNum);

    IOHIDManagerSetDeviceMatching(manager, matchDict);
    CFRelease(matchDict);

    IOReturn openRes = IOHIDManagerOpen(manager, kIOHIDOptionsTypeNone);
    printf("IOHIDManagerOpen result: 0x%08x\n", openRes);

    CFSetRef deviceSet = IOHIDManagerCopyDevices(manager);
    if (!deviceSet) {
        printf("No devices found!\n");
        return 1;
    }

    CFIndex count = CFSetGetCount(deviceSet);
    printf("Found %ld matching HID device interfaces.\n", count);

    const void *devices[count];
    CFSetGetValues(deviceSet, devices);

    for (CFIndex i = 0; i < count; i++) {
        IOHIDDeviceRef dev = (IOHIDDeviceRef)devices[i];

        int usage = 0, usagePage = 0, maxFeature = 0, maxInput = 0, maxOutput = 0;
        CFNumberRef val;

        val = (CFNumberRef)IOHIDDeviceGetProperty(dev, CFSTR(kIOHIDPrimaryUsageKey));
        if (val) CFNumberGetValue(val, kCFNumberIntType, &usage);

        val = (CFNumberRef)IOHIDDeviceGetProperty(dev, CFSTR(kIOHIDPrimaryUsagePageKey));
        if (val) CFNumberGetValue(val, kCFNumberIntType, &usagePage);

        val = (CFNumberRef)IOHIDDeviceGetProperty(dev, CFSTR(kIOHIDMaxFeatureReportSizeKey));
        if (val) CFNumberGetValue(val, kCFNumberIntType, &maxFeature);

        val = (CFNumberRef)IOHIDDeviceGetProperty(dev, CFSTR(kIOHIDMaxInputReportSizeKey));
        if (val) CFNumberGetValue(val, kCFNumberIntType, &maxInput);

        val = (CFNumberRef)IOHIDDeviceGetProperty(dev, CFSTR(kIOHIDMaxOutputReportSizeKey));
        if (val) CFNumberGetValue(val, kCFNumberIntType, &maxOutput);

        printf("\n--------------------------------------------------------\n");
        printf("Device %ld: Usage=0x%04x, Page=0x%04x | MaxFeat=%d, MaxIn=%d, MaxOut=%d\n",
               i, usage, usagePage, maxFeature, maxInput, maxOutput);

        IOReturn devOpenRet = IOHIDDeviceOpen(dev, kIOHIDOptionsTypeSeizeDevice);
        if (devOpenRet != kIOReturnSuccess) {
            devOpenRet = IOHIDDeviceOpen(dev, kIOHIDOptionsTypeNone);
        }
        printf("  IOHIDDeviceOpen result: 0x%08x\n", devOpenRet);

        // Try reading Feature Report with Report ID = 0..16
        if (maxFeature > 0) {
            for (int rId = 0; rId <= 16; rId++) {
                uint8_t readBuf[128] = {0};
                CFIndex readLen = maxFeature;
                IOReturn ret = IOHIDDeviceGetReport(dev, kIOHIDReportTypeFeature, rId, readBuf, &readLen);
                int nonZero = 0;
                for (int b = 0; b < readLen; b++) if (readBuf[b] != 0) nonZero = 1;
                if (ret == kIOReturnSuccess) {
                    if (nonZero) {
                        printf("  [!] Feature ID=%d returned NON-ZERO data: ", rId);
                        for (int b = 0; b < readLen && b < 16; b++) printf("%02x ", readBuf[b]);
                        printf("\n");
                    } else {
                        printf("  Feature ID=%d: 0x%08x (all zeroes)\n", rId, ret);
                    }
                } else {
                    printf("  Feature ID=%d: err 0x%08x\n", rId, ret);
                }
            }
        }

        // Try reading Input Report
        if (maxInput > 0) {
            uint8_t inBuf[128] = {0};
            CFIndex inLen = maxInput;
            IOReturn ret = IOHIDDeviceGetReport(dev, kIOHIDReportTypeInput, 0, inBuf, &inLen);
            printf("  IOHIDDeviceGetReport(type=Input, ID=0, len=%d): res=0x%08x, bytes_returned=%ld\n",
                   maxInput, ret, inLen);
            if (ret == kIOReturnSuccess) {
                printf("  Data: ");
                for (int b = 0; b < inLen && b < 16; b++) printf("%02x ", inBuf[b]);
                printf("\n");
            }
        }

        // If this is the vendor interface (UsagePage 0xffff, Usage 2)
        if (usagePage == 0xffff && usage == 2) {
            printf("\n  >>> TARGET VENDOR INTERFACE IDENTIFIED <<<\n");

            // Let's test sending GET_REV (0x80) and reading response
            uint8_t cmdGetRev[64] = {0};
            cmdGetRev[0] = 0x80; // FEA_CMD_GET_REV
            IOReturn sRet = IOHIDDeviceSetReport(dev, kIOHIDReportTypeFeature, 0, cmdGetRev, 64);
            printf("  Sent GET_REV (0x80): res=0x%08x\n", sRet);
            usleep(100000); // 100ms

            uint8_t respBuf[64] = {0};
            CFIndex respLen = 64;
            IOReturn gRet = IOHIDDeviceGetReport(dev, kIOHIDReportTypeFeature, 0, respBuf, &respLen);
            printf("  Read GET_REV response: res=0x%08x, len=%ld\n", gRet, respLen);
            printf("  Response bytes: ");
            for (int b = 0; b < respLen && b < 16; b++) printf("%02x ", respBuf[b]);
            printf("\n");

            // Also test sending GET_LEDPARAM (0x84 and 0x87)
            uint8_t cmdGetLed[64] = {0};
            cmdGetLed[0] = 0x84;
            IOHIDDeviceSetReport(dev, kIOHIDReportTypeFeature, 0, cmdGetLed, 64);
            usleep(100000);
            memset(respBuf, 0, sizeof(respBuf));
            respLen = 64;
            IOHIDDeviceGetReport(dev, kIOHIDReportTypeFeature, 0, respBuf, &respLen);
            printf("  Read GET_LEDPARAM (0x84) response: len=%ld, bytes: ", respLen);
            for (int b = 0; b < respLen && b < 16; b++) printf("%02x ", respBuf[b]);
            printf("\n");
        }
    }

    CFRelease(deviceSet);
    CFRelease(manager);
    return 0;
}

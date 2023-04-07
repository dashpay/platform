import { Platform } from '../../../Platform';
export declare function waitForCoreChainLockedHeight(this: Platform, expectedCoreHeight: number): Promise<{
    promise: Promise<any>;
    cancel: Function;
}>;
export default waitForCoreChainLockedHeight;

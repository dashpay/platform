import { Platform } from "../../../Platform";

import { ownerId as dpnsOwnerId } from "@dashevo/dpns-contract/lib/systemIds";

export async function waitForCoreChainLockedHeight(
  this : Platform,
  expectedCoreHeight : number,
): Promise<{ promise: Promise<any>, cancel: Function }> {
  const platform = this;
  await platform.initialize();

  const interval = 5000;

  let isCanceled = false;

  let timeout: ReturnType<typeof setTimeout>;

  let coreChainLockedHeight = 0;

  const promise = new Promise((resolve, reject) => {
    async function obtainCoreChainLockedHeight() {
      try {
        const identityResponse = await platform.identities.get(dpnsOwnerId);

        if (!identityResponse) {
           reject(new Error('Identity using to obtain core chain locked height is not present'));

           return;
        }

        const metadata = identityResponse.getMetadata();

        coreChainLockedHeight = metadata.getCoreChainLockedHeight();
      } catch (e) {
        reject(e);

        return;
      }

      if (coreChainLockedHeight >= expectedCoreHeight) {
        resolve();

        return;
      }

      if (!isCanceled) {
        timeout = setTimeout(obtainCoreChainLockedHeight, interval);
      }
    }

    obtainCoreChainLockedHeight();
  });

  function cancel() {
    if (timeout) {
      clearTimeout(timeout);
    }

    isCanceled = true;
  }

  return {
    promise,
    cancel,
  }
}

export default waitForCoreChainLockedHeight;



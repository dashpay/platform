import { Platform } from '../../../Platform';

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
        const response = await platform.client.getDAPIClient().platform.getEpochsInfo(0, 1);

        const metadata = response.getMetadata();

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
  };
}

export default waitForCoreChainLockedHeight;

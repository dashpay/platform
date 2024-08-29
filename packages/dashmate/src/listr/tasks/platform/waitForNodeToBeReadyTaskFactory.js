import DAPIClient from '@dashevo/dapi-client';
import bs58 from 'bs58';
import { Listr } from 'listr2';
import WithdrawalsContract from '@dashevo/withdrawals-contract/lib/systemIds.js';
import wait from '../../../util/wait.js';

/**
 *
 * @return {waitForNodeToBeReadyTask}
 */
export default function waitForNodeToBeReadyTaskFactory() {
  /**
   * @typedef waitForNodeToBeReadyTask
   * @param {Config} config
   * @return {Promise<void>}
   */
  async function waitForNodeToBeReadyTask(config) {
    return new Listr([
      {
        title: `Wait for node ${config.getName()} to be ready`,
        task: async () => {
          let host = config.get('platform.gateway.listeners.dapiAndDrive.host');
          const port = config.get('platform.gateway.listeners.dapiAndDrive.port');

          if (host === '0.0.0.0') {
            host = '127.0.0.1';
          }

          const dapiClient = new DAPIClient({
            dapiAddresses: [`${host}:${port}:no-ssl`],
            loggerOptions: {
              level: 'silent',
            },
          });

          const withdrawalsContractId = bs58.decode(WithdrawalsContract.contractId);

          let success = false;
          do {
            const response = await dapiClient.platform.getDataContract(withdrawalsContractId, {
              retries: 0,
              prove: false,
            })
              .catch(() => {
              });

            success = Boolean(response);

            if (!success) {
              await wait(500);
            }
          } while (!success);
        },
      },
    ]);
  }

  return waitForNodeToBeReadyTask;
}

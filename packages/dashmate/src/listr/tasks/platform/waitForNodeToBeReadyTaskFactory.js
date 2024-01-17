import DAPIClient from '@dashevo/dapi-client';
import { Listr } from 'listr2';
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
          let host = config.get('platform.dapi.envoy.http.host');
          const port = config.get('platform.dapi.envoy.http.port');

          if (host === '0.0.0.0') {
            host = '127.0.0.1';
          }

          const dapiClient = new DAPIClient({
            dapiAddresses: [`${host}:${port}:no-ssl`],
            loggerOptions: {
              level: 'silent',
            },
          });

          let success = false;
          do {
            const response = await dapiClient.platform.getEpochsInfo(0, 1, {
              retries: 0,
            })
              .catch(() => {});

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

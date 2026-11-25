import { Listr } from 'listr2';
import isServiceBuildRequired from '../../util/isServiceBuildRequired.js';

/**
 * @param {startNodeTask} startNodeTask
 * @param {stopNodeTask} stopNodeTask
 * @param {buildServicesTask} buildServicesTask
 * @return {restartNodeTask}
 */
export default function restartNodeTaskFactory(startNodeTask, stopNodeTask, buildServicesTask) {
  /**
   * Restart node
   * @typedef {restartNodeTask}
   *
   * @param {Config} config
   *
   * @return {Listr}
   */
  function restartNodeTask(config) {
    return new Listr([
      {
        enabled: () => isServiceBuildRequired(config),
        task: (ctx) => {
          ctx.skipBuildServices = true;

          return buildServicesTask(config);
        },
      },
      // Titled, because listr leaves out a subtree whose tasks all lack one,
      // and with it what stopping prints while it waits or why it failed.
      {
        title: 'Stop node',
        task: () => stopNodeTask(config),
      },
      {
        title: 'Start node',
        task: () => startNodeTask(config),
      },
    ]);
  }

  return restartNodeTask;
}

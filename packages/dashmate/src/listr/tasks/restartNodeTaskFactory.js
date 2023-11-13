import { Listr }  from 'listr2';
import {isServiceBuildRequired} from "../../util/isServiceBuildRequired.js";

/**
 * @param {startNodeTask} startNodeTask
 * @param {stopNodeTask} stopNodeTask
 * @param {buildServicesTask} buildServicesTask
 * @return {restartNodeTask}
 */
export function restartNodeTaskFactory(startNodeTask, stopNodeTask, buildServicesTask) {
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
      {
        task: () => stopNodeTask(config),
      },
      {
        task: () => startNodeTask(config),
      },
    ]);
  }

  return restartNodeTask;
}

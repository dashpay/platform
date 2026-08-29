import { Listr } from 'listr2';
import wireLocalTenderdashNode from './wireLocalTenderdashNode.js';

/**
 * @return {configureTenderdashTask}
 */
export default function configureTenderdashTaskFactory() {
  /**
   * @typedef {configureTenderdashTask}
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function configureTenderdashTask(configGroup) {
    return new Listr([
      {
        task: async () => {
          const platformConfigs = configGroup.filter((config) => config.get('platform.enable'));

          const subTasks = [];

          // Interconnect Tenderdash nodes
          subTasks.push({
            task: async () => {
              const randomChainIdPart = Math.floor(Math.random() * 60) + 1;
              const chainId = `dashmate_local_${randomChainIdPart}`;

              platformConfigs.forEach((config) => {
                wireLocalTenderdashNode(config, chainId, platformConfigs);
              });
            },
          });

          return new Listr(subTasks);
        },
      },
    ]);
  }

  return configureTenderdashTask;
}

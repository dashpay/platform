import chalk from 'chalk';
import { Listr } from 'listr2';

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {getServiceList} getServiceList
 * @param {verifySystemRequirements} verifySystemRequirements
 * @return {analyseSamplesTask}
 */
export default function analyseSamplesTaskFactory(
  dockerCompose,
  getServiceList,
  verifySystemRequirements,
) {
  /**
   * @typedef {function} analyseSamplesTask
   * @param config
   * @return {Listr}
   */
  function analyseSamplesTask(config) {
    return new Listr([
      {
        title: 'System resources',
        task: async (ctx) => {
          const {
            cpu,
            dockerSystemInfo,
            currentLoad,
            diskSpace,
            fsOpenFiles,
            memory,
          } = ctx.samples.getSystemInfo();

          ctx.systemResourceProblems = verifySystemRequirements(
            {
              dockerSystemInfo,
              cpu,
              memory,
              diskSpace,
            },
            config.get('platform.enabled'),
            {
              diskSpace: 5,
            },
          );

          return new Listr([
            {
              title: 'CPU cores',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.cpuCores) {
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'CPU speed',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.cpuSpeed) {
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'CPU load',
              task: (_ctx, task) => {
                if (currentLoad.avgLoad > 0.8) {
                  ctx.systemResourceProblems.avgLoad = `Average system load ${currentLoad.avgLoad.toFixed(2)} is higher than normal. Consider to upgrade CPU.`;
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Total RAM',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.memory) {
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Free RAM',
              enabled: Number.isInteger(memory.free),
              task: (_ctx, task) => {
                if (memory.free) {
                  const memoryGb = memory.free / (1024 ** 3);
                  if (memoryGb < 0.5) {
                    ctx.systemResourceProblems.freeMemory = `Only ${memoryGb.toFixed(1)}GB RAM is available. Consider to upgrade RAM.`;
                    throw new Error(task.title);
                  }
                }
              },
            },
            {
              title: 'File descriptors',
              enabled: fsOpenFiles?.allocated && fsOpenFiles?.max,
              task: (_ctx, task) => {
                const available = fsOpenFiles.max - fsOpenFiles.allocated;
                if (available < 1000) {
                  ctx.systemResourceProblems.fsOpenFiles = `${available} available file descriptors left. Consider to increase max limit.`;
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Swap',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.swap) {
                  throw new Error(task.title);
                }
              },
            },
            {
              title: 'Disk space',
              task: (_ctx, task) => {
                if (ctx.systemResourceProblems.diskSpace) {
                  throw new Error(task.title);
                }
              },
            },
            // TODO: Disk IO
          ], {
            exitOnError: false,
          });
        },
      },
      {
        title: 'Docker is started',
        task: async (ctx, task) => {
          try {
            await dockerCompose.throwErrorIfNotInstalled();
          } catch (e) {
            ctx.problems.push(e.message);
            ctx.skipOthers = true;

            throw new Error(task.title);
          }
        },
      },
      {
        title: 'Services are started',
        skip: (ctx) => ctx.skipOthers,
        task: async (ctx) => {
          const services = getServiceList(config);

          ctx.servicesNotStarted = [];
          ctx.servicesFailed = [];
          ctx.servicesOOMKilled = [];

          return new Listr(
            services.map((service) => (
              {
                title: service.title,
                task: () => {
                  const dockerInspect = ctx.samples.getServiceInfo(service.name, 'dockerInspect');

                  if (dockerInspect.message) {
                    ctx.servicesNotStarted.push({
                      service,
                      message: dockerInspect.message,
                    });
                  } else if (
                    dockerInspect.State.Restarting === true
                    && dockerInspect.State.ExitCode !== 0
                  ) {
                    ctx.servicesFailed.push({
                      service,
                      message: dockerInspect.State.Error,
                      code: dockerInspect.State.ExitCode,
                    });
                  } else if (dockerInspect.State.OOMKilled === true) {
                    ctx.servicesOOMKilled.push({
                      service,
                    });
                  } else {
                    return;
                  }

                  throw new Error(service.title);
                },
              }
            )),
            {
              exitOnError: false,
            },
          );
        },
      },
      {
        skip: (ctx) => ctx.skipOthers,
        task: async (ctx) => {
          if (ctx.servicesNotStarted.length > 0) {
            let problem;
            if (ctx.servicesNotStarted.length === 1) {
              problem = chalk`Service ${ctx.servicesNotStarted[0].service.title} isn't started.`
            } else {
              problem = chalk`Services ${ctx.servicesNotStarted.map((e) => e.service.title).join(', ')} aren't started.`
            }

            problem += chalk`\n\nTry {bold.blueBright dashmate start --force} to make sure all services are started`;

            ctx.problems.push(problem);
          }

          if (ctx.servicesFailed.length > 0) {
            let problem;
            if (ctx.servicesFailed.length === 1) {
              const failedService = ctx.servicesFailed[0];

              problem = chalk`Service ${failedService.service.title} failed with an error code ${failedService.code}`;

              if (failedService.message?.length > 0) {
                problem += `and message: ${failedService.message}`;
              }

              problem += '.';
            } else {
              problem = chalk`${ctx.servicesFailed.length} services failed:`;

              ctx.servicesFailed.map((failedService) => {
                let output = chalk`  ${failedService.service.title} failed with an error code ${failedService.code}`;

                if (failedService.message?.length > 0) {
                  output += `and message: ${failedService.message}`;
                }

                output += '.';

                return output;
              }).join('\n');
            }

            problem += chalk`\n\nPlease check corresponding logs or share them with Dash Core Group`;

            ctx.problems.push(problem);
          }

          if (ctx.servicesOOMKilled.length > 0) {
            let problem;
            if (ctx.servicesNotStarted.length === 1) {
              problem = chalk`Service ${ctx.servicesNotStarted[0].service.title} is killed due to lack of memory.`;
            } else {
              problem = chalk`Services ${ctx.servicesNotStarted.map((e) => e.service.title).join(', ')} aren't killed due to lack of memory.`;
            }

            problem += chalk`\n\nMake sure you have enough memory to run the node.`;

            ctx.problems.push(problem);
          }

          if (ctx.systemResourceProblems.length > 0) {

          }
        },
      },
      // TODO: dont have priavate ky to sign
      {
        title: 'Services are started',
        skip: (ctx) => ctx.skipOthers,
        task: async (ctx, task) => {

          // TODO: metrics enabled but admin is not
        },
      },
    ], {
      exitOnError: false,
    });
  }

  return analyseSamplesTask;
}

import chalk from 'chalk';
import { Listr } from 'listr2';

/**
 *
 * @param {DockerCompose} dockerCompose
 * @param {getServiceList} getServiceList
 * @return {analyseSamplesTask}
 */
export default function analyseSamplesTaskFactory(dockerCompose, getServiceList) {
  /**
   * @typedef {function} analyseSamplesTask
   * @param config
   * @return {Listr}
   */
  function analyseSamplesTask(config) {
    return new Listr([
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
            services.map((service) => {
              return {
                title: service.title,
                task: () => {
                  const dockerInspect = ctx.samples.getServiceInfo(service.name, 'dockerInspect');

                  if (dockerInspect.message) {
                    ctx.servicesNotStarted.push({
                      service,
                      message: dockerInspect.message,
                    });
                  } else if (dockerInspect.State.Restarting) {
                    // TODO: ctx.servicesFailed
                    //dockerInspect.State.Started = dockerInspect.State.Started ?? false;
                  } else {
                    return;
                  }

                  throw new Error(service.title);
                },
              };
            }),
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

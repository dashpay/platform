const { Listr } = require('listr2');
const fs = require('fs');

/**
 * @param {createCertificate} createCertificate
 * @param {saveChallenge} saveChallenge
 * @param {verifyTempServer} verifyTempServer
 * @param {verifyDomain} verifyDomain
 * @param {downloadCertificate} downloadCertificate
 * @param {Docker} docker
 * @param {string} homeDirPath
 * @return {createZerosslCertificateTask}
 */
function createZerosslCertificateTaskFactory(
  createCertificate,
  saveChallenge,
  verifyTempServer,
  verifyDomain,
  downloadCertificate,
  docker,
  homeDirPath,
) {
  /**
   * @typedef {createZerosslCertificateTask}
   * @param {Config} config
   * @return {Listr}
   */
  async function createZerosslCertificateTask(config) {
    return new Listr([
      {
        title: 'Read CSR',
        task: async (ctx) => {
          ctx.csr = fs.readFileSync(`${homeDirPath}/ssl/domain.csr`, 'utf8');
        },
      },
      {
        title: 'Request certificate challenge',
        task: async (ctx) => {
          ctx.response = await createCertificate(ctx.csr, config);
          if ('error' in ctx.response.data) {
            throw new Error(ctx.response.data.error.type);
          }
        },
      },
      {
        title: 'Save challenge',
        task: async (ctx, task) => {
          ({
            validationFile: ctx.challengePath,
            fileName: ctx.challengeFile,
          } = await saveChallenge(ctx.response, homeDirPath, config));
          // eslint-disable-next-line no-param-reassign
          task.output = `Challenge saved: ${ctx.challengePath}`;
        },
        options: { persistentOutput: true },
      },
      {
        title: 'Set up temp server',
        task: async (ctx) => {
          try {
            // Why doesn't any of this work???

            /*
            docker.pull('nginx');

            await docker.pull('nginx');

            docker.pull('nginx', function (err, stream) {
              docker.modem.followProgress(stream, onFinished);
              function onFinished(err, output) {
                console.log('done pulling');
              }
            });
            */

            const opts = {
              name: 'mn-ssl-verification',
              Image: 'nginx',
              Tty: false,
              HostConfig: {
                AutoRemove: true,
                Binds: [`${homeDirPath}/ssl:/usr/share/nginx/html:ro`],
                PortBindings: { '80/tcp': [{ HostPort: '80' }] },
              },
            };

            ctx.nginx = await docker.createContainer(opts);
            await ctx.nginx.start();
          } catch (e) {
            throw new Error(e);
          }

          await verifyTempServer(
            ctx.challengeFile,
            config.get('externalIp'),
          );
        },
      },
      {
        title: 'Verify IP',
        task: async (ctx) => {
          try {
            await verifyDomain(ctx.response.data.id, config);
          } catch (error) {
            throw new Error(error);
          }
        },
      },
      {
        title: 'Download certificate',
        task: async (ctx) => {
          await downloadCertificate(ctx.response.data.id, homeDirPath, config);
        },
      },
      {
        title: 'Stop temp server',
        task: async (ctx) => {
          await ctx.nginx.stop();
          fs.rmdir(`${homeDirPath}/ssl/.well-known`, { recursive: true }, (error) => {
            if (error) {
              throw new Error(error);
            }
          });
        },
      },
    ]);
  }

  return createZerosslCertificateTask;
}

module.exports = createZerosslCertificateTaskFactory;

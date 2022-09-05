const { Listr } = require('listr2');

/**
 * @param {generateKeyPair} generateKeyPair
 * @param {generateCsr} generateCsr
 * @param {createSelfSignedCertificate} createSelfSignedCertificate
 * @param {setupCertificateTask} setupCertificateTask
 * @return {configureSelfSignedCertificatesTask}
 */
function configureSelfSignedCertificatesTaskFactory(
  generateKeyPair,
  generateCsr,
  createSelfSignedCertificate,
  setupCertificateTask,
) {
  /**
   * @typedef {configureSelfSignedCertificatesTask}
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function configureSelfSignedCertificatesTask(configGroup) {
    return new Listr([
      {
        task: async (ctx) => {
          const platformConfigs = configGroup.filter((config) => config.has('platform'));

          const subTasks = platformConfigs.map((config) => ({
            title: `Create certificate for ${config.getName()}`,
            task: async () => {
              ctx.keyPair = await generateKeyPair();
              ctx.csr = await generateCsr(ctx.keyPair, config.get('externalIp', true));
              ctx.certificate = await createSelfSignedCertificate(ctx.keyPair, ctx.csr);

              config.set('platform.dapi.envoy.ssl.selfSigned', true);

              return setupCertificateTask(config);
            },
          }));

          return new Listr(subTasks);
        },
      },
    ]);
  }

  return configureSelfSignedCertificatesTask;
}

module.exports = configureSelfSignedCertificatesTaskFactory;

const { Listr } = require('listr2');

/**
 * @param {generateKeyPair} generateKeyPair
 * @param {generateCsr} generateCsr
 * @param {createSelfSignedCertificate} createSelfSignedCertificate
 * @param {saveCertificateTask} saveCertificateTask
 * @return {obtainSelfSignedCertificateTask}
 */
function obtainSelfSignedCertificateTaskFactory(
  generateKeyPair,
  generateCsr,
  createSelfSignedCertificate,
  saveCertificateTask,
) {
  /**
   * @typedef {obtainSelfSignedCertificateTask}
   * @param {Config[]} configGroup
   * @return {Listr}
   */
  function obtainSelfSignedCertificateTask(configGroup) {
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

              config.set('platform.dapi.envoy.ssl.provider', 'selfSigned');

              return saveCertificateTask(config);
            },
          }));

          return new Listr(subTasks);
        },
      },
    ]);
  }

  return obtainSelfSignedCertificateTask;
}

module.exports = obtainSelfSignedCertificateTaskFactory;

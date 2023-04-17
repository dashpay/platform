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
   * @param {Config} config
   * @return {Listr}
   */
  function obtainSelfSignedCertificateTask(config) {
    return new Listr([
      {
        task: async (ctx) => {
          ctx.keyPair = await generateKeyPair();
          ctx.csr = await generateCsr(ctx.keyPair, config.get('externalIp', true));
          ctx.certificate = await createSelfSignedCertificate(ctx.keyPair, ctx.csr);

          return saveCertificateTask(config);
        },
      },
    ]);
  }

  return obtainSelfSignedCertificateTask;
}

module.exports = obtainSelfSignedCertificateTaskFactory;

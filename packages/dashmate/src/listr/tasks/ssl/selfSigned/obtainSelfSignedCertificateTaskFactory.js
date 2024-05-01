import { Listr } from 'listr2';

/**
 * @param {generateKeyPair} generateKeyPair
 * @param {generateCsr} generateCsr
 * @param {createSelfSignedCertificate} createSelfSignedCertificate
 * @param {saveCertificateTask} saveCertificateTask
 * @return {obtainSelfSignedCertificateTask}
 */
export default function obtainSelfSignedCertificateTaskFactory(
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
          ctx.privateKeyFile = ctx.keyPair.privateKey;
          ctx.csr = await generateCsr(ctx.keyPair, config.get('externalIp', true));

          ctx.certificateFile = await createSelfSignedCertificate(ctx.keyPair, ctx.csr);

          return saveCertificateTask(config);
        },
      },
    ]);
  }

  return obtainSelfSignedCertificateTask;
}

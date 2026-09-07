import fs from 'fs';
import { Listr } from 'listr2';

import promptOrThrow from '../../../util/promptOrThrow.js';
import validateFileExists from '../../prompts/validators/validateFileExists.js';
import validateSslCertificateFiles from '../../prompts/validators/validateSslCertificateFiles.js';

/**
 * @param {saveCertificateTask} saveCertificateTask
 * @return {installCertificateFilesTask}
 */
export default function installCertificateFilesTaskFactory(saveCertificateTask) {
  /**
   * Ask for a certificate chain and key on disk and install them for the
   * gateway.
   *
   * Shared by the setup wizard's "File on disk" provider and by the update
   * certificate check, which offers an operator with their own certificate the
   * chance to replace it before it suggests changing authority.
   *
   * @typedef {installCertificateFilesTask}
   * @param {Config} config
   * @param {Object} [options]
   * @param {boolean} [options.interactive]
   * @return {Listr}
   */
  function installCertificateFilesTask(config, { interactive } = {}) {
    return new Listr([
      {
        title: 'Set SSL certificate file',
        task: async (ctx, task) => {
          let form = ctx.fileCertificateProviderForm;

          if (!ctx.fileCertificateProviderForm) {
            form = await promptOrThrow(task, {
              type: 'form',
              header: `  To configure SSL certificates, you need to provide a certificate chain file
  and a private key file.
  The certificate chain file should contain your server certificate at the top and
  then intermediate/root certificates if present.\n`,
              message: 'Specify paths to your certificate files',
              choices: [
                {
                  name: 'chainFilePath',
                  message: 'Path to certificate chain file',
                  validate: validateFileExists,
                },
                {
                  name: 'privateFilePath',
                  message: 'Path to certificate key file',
                  validate: validateFileExists,
                },
              ],
              validate: ({ chainFilePath, privateFilePath }) => {
                if (!validateFileExists(chainFilePath)) {
                  return 'certificate chain file path is not valid';
                }

                if (!validateFileExists(privateFilePath)) {
                  return 'certificate key file path is not valid';
                }

                if (chainFilePath === privateFilePath) {
                  return 'the same path for both files';
                }

                const isValid = validateSslCertificateFiles(chainFilePath, privateFilePath);

                if (!isValid) {
                  return 'The certificate and private key do not match';
                }

                return true;
              },
            }, { interactive });
          }

          ctx.certificateFile = fs.readFileSync(form.chainFilePath, 'utf8');
          ctx.privateKeyFile = fs.readFileSync(form.privateFilePath, 'utf8');

          return saveCertificateTask(config);
        },
      },
    ]);
  }

  return installCertificateFilesTask;
}

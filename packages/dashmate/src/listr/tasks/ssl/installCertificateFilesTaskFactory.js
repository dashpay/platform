import { Listr } from 'listr2';

import promptOrThrow from '../../../util/promptOrThrow.js';
import validateFileExists from '../../prompts/validators/validateFileExists.js';
import {
  checkCertificatePair,
  readCertificateFilePair,
} from '../../prompts/validators/validateCertificateFilePaths.js';

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
   * What is installed is what was checked: the pair a caller read and checked
   * in advance (ctx.validatedCertificatePair), or else the files read once
   * each here and checked before anything is written.
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
          let pair = ctx.validatedCertificatePair;

          if (!pair && ctx.fileCertificateProviderForm) {
            pair = readCertificateFilePair(ctx.fileCertificateProviderForm);
          }

          if (!pair) {
            // The pair the form's check accepted is the one installed, not
            // the files as they are once the form returns.
            let acceptedPair;

            await promptOrThrow(task, {
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
              validate: (paths) => {
                try {
                  acceptedPair = readCertificateFilePair(paths);
                } catch (e) {
                  return e.message;
                }

                return true;
              },
            }, { interactive });

            pair = acceptedPair;
          }

          // Checked again as it is installed: a pair checked in advance waited
          // for Core to sync, which can outlast what was left of its validity.
          checkCertificatePair(pair);

          ctx.certificateFile = pair.certificateFile;
          ctx.privateKeyFile = pair.privateKeyFile;

          return saveCertificateTask(config);
        },
      },
    ]);
  }

  return installCertificateFilesTask;
}

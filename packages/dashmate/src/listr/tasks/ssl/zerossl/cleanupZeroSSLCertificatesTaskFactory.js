import chalk from 'chalk';
import { Listr } from 'listr2';
import { Observable } from 'rxjs';
import wait from '../../../../util/wait.js';

/**
 * @param {listCertificates} listCertificates
 * @param {cancelCertificate} cancelCertificate
 * @return {cleanupZeroSSLCertificatesTask}
 */
export default function cleanupZeroSSLCertificatesTaskFactory(
  listCertificates,
  cancelCertificate,
) {
  /**
   * @typedef {cleanupZeroSSLCertificatesTask}
   * @param {Config} config
   * @return {Listr}
   */
  function cleanupZeroSSLCertificatesTask(config) {
    const apiKey = config.get('platform.gateway.ssl.providerConfigs.zerossl.apiKey', true);

    return new Listr([
      {
        title: 'Collect drafted and pending validation certificates',
        // Skips the check if force flag is set
        task: async (ctx, task) => {
          ctx.certificates = [];

          let certificatesPerRequest = [];
          let page = 1;

          // Fetch all certificates in draft or pending validation status
          // with pagination
          do {
            certificatesPerRequest = await listCertificates(apiKey, ['draft', 'pending_validation'], page);

            ctx.certificates = ctx.certificates.concat(certificatesPerRequest);

            page += 1;

            // eslint-disable-next-line no-param-reassign
            task.output = `Found ${ctx.certificates.length} certificates`;
          } while (certificatesPerRequest.length === 1000);

          ctx.total = ctx.certificates.length;
        },
      },
      {
        title: 'Cancel certificates',
        skip: (ctx) => ctx.certificates.length === 0,
        task: async (ctx, task) => {
          // eslint-disable-next-line no-param-reassign
          task.title = `Cancel ${ctx.certificates.length} certificates`;
          ctx.canceled = 0;
          ctx.errored = 0;
          return new Observable(async (observer) => {
            for (const certificate of ctx.certificates) {
              try {
                await cancelCertificate(apiKey, certificate.id);

                ctx.canceled += 1;
              } catch (e) {
                ctx.errored += 1;

                if (process.env.DEBUG) {
                  // eslint-disable-next-line no-console
                  console.warn(e);
                }
              }

              observer.next(chalk`{green ${ctx.canceled}} / {red ${ctx.errored}} / ${ctx.total}`);

              await wait(100);
            }

            if (ctx.errored > 0) {
              observer.error(new Error('Some certificates were not canceled. Please try again.'));
            } else {
              observer.complete();
            }

            return this;
          });
        },
        options: {
          persistentOutput: true,
        },
      },
    ], {
      rendererOptions: {
        showErrorMessage: true,
      },
    });
  }

  return cleanupZeroSSLCertificatesTask;
}

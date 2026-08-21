import { Listr } from 'listr2';
import fs from 'fs';
import path from 'path';
import os from 'os';

import { ERRORS } from '../../../../ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';
import LegoCertificate from '../../../../ssl/letsencrypt/LegoCertificate.js';
import { LETSENCRYPT_ACME_DIRECTORY_URL } from '../../../../constants.js';
import LegoArtifactsMissingError from '../../../../ssl/errors/LegoArtifactsMissingError.js';
import LegoDidNotStartError from '../../../../ssl/errors/LegoDidNotStartError.js';
import LegoResultNotObservedError from '../../../../ssl/errors/LegoResultNotObservedError.js';
import promptOrThrow from '../../../../util/promptOrThrow.js';
import renderConfigFlag from '../../../../util/renderConfigFlag.js';

const LEGO_IMAGE = 'goacme/lego:v4.31.0';

/**
 * Let's Encrypt allows five failed authorizations per address per account per
 * hour, and that budget is shared with the helper's renewal of a still-valid
 * certificate, so an attempt is not free. Three is enough for an operator who
 * is fixing a firewall rule between attempts and few enough to leave the
 * helper room.
 */
const MAX_OBTAIN_ATTEMPTS = 3;

/**
 * Port 80 is a standing requirement, not a step.
 *
 * An IP-address certificate lasts about six days and every renewal performs a
 * fresh challenge, so a rule opened once for a migration and closed afterwards
 * - or one that does not survive a reboot - takes the node dark within a week,
 * and nothing reports it. The operator who has just succeeded is the one least
 * likely to hear this otherwise, because they never saw a failure.
 */
export const PORT_80_PERMANENCE = `LEAVE PORT 80 OPEN. This is not a one-time requirement. Certificates for
IP addresses last about six days, and dashmate keeps renewing this one for as
long as the node runs - every renewal needs inbound port 80 again.

If you opened port 80 just to make this work, make the rule permanent and make
sure it survives a reboot. If it lapses, this node goes dark within six days
and nothing will tell you.`;

/**
 * What to tell an operator who has run out of attempts.
 *
 * No claim about when to come back: a long-failing address may be paused rather
 * than rate-limited, and waiting never clears a pause - which is exactly the
 * state a node that has been dark for months is likely to be in.
 *
 * @param {Config} config
 * @param {number} attempts
 * @return {string}
 */
function renderGiveUpGuidance(config, attempts) {
  return `dashmate did not obtain a certificate after ${attempts} `
    + `attempt${attempts === 1 ? '' : 's'}.

Retrying now also blocks this node's automatic renewal: dashmate's helper
renews under the same Let's Encrypt account, and failed attempts are shared.

If this node has been failing for a long time, the address may be PAUSED
rather than rate-limited - waiting does not clear a pause, and you may need
Let's Encrypt's Self-Service Portal to unpause it:
    https://letsencrypt.org/docs/rate-limits/

Fix inbound port 80 first, then: `
    + `dashmate ssl obtain ${renderConfigFlag(config.getName())} --provider letsencrypt`
    + `\n\n${PORT_80_PERMANENCE}`;
}

/**
 * What to tell an operator whose helper never started.
 *
 * Nothing reached the certificate authority, so none of the authority-side
 * consequences apply: nothing was validated, no issuance budget was spent, and
 * no address can have been paused. Saying otherwise would send them to a
 * rate-limit portal over a local port conflict.
 *
 * @param {Config} config
 * @param {Error} cause
 * @return {string}
 */
function renderHelperDidNotStartGuidance(config, cause) {
  return `dashmate could not start the certificate helper, so no request was
made to Let's Encrypt. Nothing was issued, nothing was validated, and no
rate limit was spent.

Docker reported:

${cause.message}

That message is the diagnosis - dashmate did not look further than it. One
common cause is another process already holding port 80, which is the
opposite of a blocked port: it is reachable and occupied. Others are the
Docker daemon being unreachable, or the current user not being permitted to
use it.

    sudo ss -lntp 'sport = :80'
    dashmate ssl obtain ${renderConfigFlag(config.getName())} --provider letsencrypt`;
}

/**
 * What to tell an operator whose certificate was issued but never landed.
 *
 * The issuance is the fact that matters: it is spent whether or not the files
 * arrived, so the one thing this must not do is invite another attempt.
 *
 * @param {Config} config
 * @param {string} missingPath
 * @return {string}
 */
function renderArtifactsMissingGuidance(config, missingPath) {
  return `Let's Encrypt issued a certificate, but dashmate could not find the
file it should have written:

    ${missingPath}

That certificate counted against this node's issuance limit - five per address
per week - and it cannot be recovered: a file that was never written cannot be
read back, and the authority does not re-send it. Running the command again
requests a replacement, which spends the limit a second time.

So fix the local cause first. Check the disk for space and for permissions, and
that the helper is allowed to write there. Only then:

    dashmate ssl obtain ${renderConfigFlag(config.getName())} --provider letsencrypt`;
}

/**
 * What to tell an operator when the helper ran and its result was never read.
 *
 * A request may have been made, so it would be wrong to say nothing reached the
 * authority - and nothing was read, so it is equally wrong to report what the
 * authority said. Both claims are withheld and the state is described instead.
 *
 * @param {Config} config
 * @param {Error} cause
 * @return {string}
 */
function renderResultNotObservedGuidance(config, cause) {
  return `The certificate helper started, but dashmate could not read how it
finished:

${cause.message}

So dashmate does not know whether a certificate was requested. Check whether
one arrived before trying again - a request that did reach Let's Encrypt
counts against this node's limits whether or not dashmate saw the answer:

    dashmate doctor ${renderConfigFlag(config.getName())}
    dashmate ssl obtain ${renderConfigFlag(config.getName())} --provider letsencrypt`;
}

const LEGO_CA_CERTIFICATE_MOUNT_PATH = '/acme-ca.pem';

/**
 * @param {Docker} docker
 * @param {dockerPull} dockerPull
 * @param {StartedContainers} startedContainers
 * @param {HomeDir} homeDir
 * @param {validateLetsEncryptCertificate} validateLetsEncryptCertificate
 * @param {saveCertificateTask} saveCertificateTask
 * @param {string|null} legoCaCertificatePath - CA that signed the ACME
 *   directory's own certificate, for a directory that is not publicly trusted
 * @param {Object} legoContainerOptions - Docker create-container overrides for
 *   the lego container, so it can be reached over a network of the caller's
 *   choosing rather than the host's port 80
 * @return {obtainLetsEncryptCertificateTask}
 */
export default function obtainLetsEncryptCertificateTaskFactory(
  docker,
  dockerPull,
  startedContainers,
  homeDir,
  validateLetsEncryptCertificate,
  saveCertificateTask,
  legoCaCertificatePath,
  legoContainerOptions,
) {
  /**
   * Create and start the lego container, reporting a failure to do either as
   * distinct from a failure the certificate authority returned.
   *
   * Nothing here has spoken to the authority yet, so a failure means no
   * validation was attempted and no issuance budget was spent.
   *
   * @param {Object} options - Docker create-container options
   * @return {Promise<Object>} the started container
   */
  async function startLegoContainer(options) {
    let container;

    try {
      container = await docker.createContainer(options);
      await container.start();
    } catch (e) {
      throw new LegoDidNotStartError(e);
    }

    return container;
  }

  /**
   * @typedef {obtainLetsEncryptCertificateTask}
   * @param {Config} config
   * @return {Listr}
   */
  function obtainLetsEncryptCertificateTask(config) {
    return new Listr([
      {
        title: 'Initialize configuration',
        task: async (ctx) => {
          // Always load config values (needed even when --force is used)
          ctx.email = config.get('platform.gateway.ssl.providerConfigs.letsencrypt.email');
          ctx.acmeDirectoryUrl = config.get('platform.gateway.ssl.providerConfigs.letsencrypt.acmeDirectoryUrl');
          ctx.externalIp = config.get('externalIp');
          ctx.legoDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'lego');
          ctx.sslConfigDir = homeDir.joinPath(config.getName(), 'platform', 'gateway', 'ssl');
          ctx.configurationUpdateRequired = !config.get('platform.gateway.ssl.enabled')
            || config.get('platform.gateway.ssl.provider') !== 'letsencrypt';

          if (!ctx.externalIp) {
            throw new Error('External IP is not set. Please set it in the config file');
          }

          // Ensure lego directories exist
          fs.mkdirSync(ctx.legoDir, { recursive: true });
          fs.mkdirSync(path.join(ctx.legoDir, 'certificates'), { recursive: true });
          fs.mkdirSync(path.join(ctx.legoDir, 'accounts'), { recursive: true });

          // Set paths
          ctx.legoCertPath = path.join(ctx.legoDir, 'certificates', `${ctx.externalIp}.crt`);
          ctx.legoKeyPath = path.join(ctx.legoDir, 'certificates', `${ctx.externalIp}.key`);

          // When force is used, skip validation and obtain new certificate
          if (ctx.force) {
            ctx.certificateValid = false;
            ctx.isRenewal = false;
          }
        },
      },
      {
        title: 'Check if certificate already exists and is valid',
        skip: (ctx) => ctx.force,
        task: async (ctx, task) => {
          const expirationDays = ctx.expirationDays ?? LegoCertificate.EXPIRATION_LIMIT_DAYS;
          const { error, data } = await validateLetsEncryptCertificate(config, expirationDays);

          // Merge validation data (but don't overwrite already-set values)
          Object.keys(data).forEach((key) => {
            if (ctx[key] === undefined) {
              ctx[key] = data[key];
            }
          });

          switch (error) {
            case undefined:
              ctx.certificateValid = true;
              // eslint-disable-next-line no-param-reassign
              task.output = `Certificate is valid and expires at ${ctx.certificate.expires}`;
              break;
            case ERRORS.EXTERNAL_IP_IS_NOT_SET:
              throw new Error('External IP is not set. Please set it in the config file');
            case ERRORS.CERTIFICATE_NOT_FOUND:
              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate not found, obtaining a new one';
              ctx.certificateValid = false;
              ctx.isRenewal = false;
              break;
            case ERRORS.PRIVATE_KEY_NOT_FOUND:
              // eslint-disable-next-line no-param-reassign
              task.output = 'Private key not found, obtaining a new certificate';
              ctx.certificateValid = false;
              ctx.isRenewal = false;
              break;
            case ERRORS.CERTIFICATE_EXPIRES_SOON:
              // eslint-disable-next-line no-param-reassign
              task.output = `Certificate expires soon at ${ctx.certificate.expires}, renewing`;
              ctx.certificateValid = false;
              ctx.isRenewal = true;
              break;
            case ERRORS.CERTIFICATE_IP_MISMATCH:
              throw new Error(`Certificate does not match external IP ${ctx.externalIp}.\n`
                + 'Please change the external IP in config or use --force to obtain a new certificate.');
            case ERRORS.CERTIFICATE_NOT_VALID:
              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate is not valid, obtaining a new one';
              ctx.certificateValid = false;
              ctx.isRenewal = false;
              break;
            case ERRORS.CERTIFICATE_NOT_INSTALLED:
              // The certificate itself is fine; it just never reached the
              // files the gateway loads. Issuing another one would spend an
              // issuance to fix a copy, and the helper schedules this same
              // path whenever the pair is not installed - so without this case
              // an affected node retries hourly and throws every time.
              // eslint-disable-next-line no-param-reassign
              task.output = 'Certificate is valid but not installed for the gateway';
              ctx.certificateValid = true;
              break;
            default:
              throw new Error(`Unknown error: ${error}`);
          }
        },
      },
      {
        title: `Pull lego Docker image (${LEGO_IMAGE})`,
        skip: (ctx) => ctx.certificateValid,
        task: async () => {
          await dockerPull(LEGO_IMAGE);
        },
      },
      {
        title: 'Obtain certificate using lego',
        skip: (ctx) => ctx.certificateValid,
        task: async (ctx, task) => {
          const { uid, gid } = os.userInfo();

          let acmeDirectoryUrl;
          try {
            acmeDirectoryUrl = new URL(ctx.acmeDirectoryUrl);
          } catch {
            throw new Error('ACME directory URL must use HTTPS');
          }

          if (acmeDirectoryUrl.protocol !== 'https:') {
            throw new Error('ACME directory URL must use HTTPS');
          }

          // Named before the request rather than after it. Until now the
          // directory only appeared inside lego's own output, so a node
          // pointed at staging - or at production when staging was meant -
          // could not be told apart until an authorization had been spent.
          const isProductionDirectory = acmeDirectoryUrl.toString()
            === LETSENCRYPT_ACME_DIRECTORY_URL;

          // eslint-disable-next-line no-param-reassign
          task.output = `Certificate authority: ${acmeDirectoryUrl.toString()}`
            + `${isProductionDirectory ? '' : ' (NOT the production directory)'}`;

          // Determine if this is initial run or renewal
          const command = ctx.isRenewal ? 'renew' : 'run';

          // Build lego command arguments
          // --disable-cn is needed for IP address certificates
          // --key-type rsa2048 is needed because node-forge doesn't support ECDSA
          // lego keys its on-disk ACME account directory by the contact
          // address, so an empty --email is a different account from no
          // --email at all. Nothing asks for one any more, and RFC 8555 makes
          // the contact optional, so an unset value omits the argument rather
          // than passing it empty.
          const legoArgs = [
            `--server=${acmeDirectoryUrl.toString()}`,
            ...(ctx.email ? ['--email', ctx.email] : []),
            '--accept-tos',
            '--http',
            '--http.port', ':80',
            '--domains', ctx.externalIp,
            '--disable-cn',
            '--key-type', 'rsa2048',
            '--path', '/data',
            command,
          ];

          // shortlived profile is required for IP address certificates
          legoArgs.push('--profile', 'shortlived');

          if (ctx.isRenewal) {
            legoArgs.push('--days', '30');
          }

          const containerName = 'dashmate-letsencrypt-lego';

          const runLego = async () => {
            // Clearing a stale container from a previous run happens before
            // lego exists, so a failure here is as far from a response by the
            // certificate authority as a refused port binding is.
            try {
              const existingContainer = await docker.getContainer(containerName);
              await existingContainer.remove({ force: true });

              try {
                await existingContainer.wait();
              } catch (waitError) {
                // Skip error if container is already removed
                if (waitError.statusCode !== 404) {
                  throw waitError;
                }
              }
            } catch (e) {
              // Container doesn't exist, that's fine
              if (e.statusCode !== 404) {
                throw new LegoDidNotStartError(e);
              }
            }

            const binds = [`${ctx.legoDir}:/data`];
            const env = [];

            // An ACME directory that is not publicly trusted - a staging or local
            // server - presents a certificate lego rejects unless told which CA
            // signed it.
            if (legoCaCertificatePath) {
              binds.push(`${legoCaCertificatePath}:${LEGO_CA_CERTIFICATE_MOUNT_PATH}:ro`);
              env.push(`LEGO_CA_CERTIFICATES=${LEGO_CA_CERTIFICATE_MOUNT_PATH}`);
            }

            // From here to the container running, any failure means the helper
            // never ran and nothing reached the authority.
            const container = await startLegoContainer({
              name: containerName,
              Image: LEGO_IMAGE,
              Cmd: legoArgs,
              Env: env,
              User: `${uid}:${gid}`,
              ExposedPorts: { '80/tcp': {} },
              ...legoContainerOptions,
              HostConfig: {
                AutoRemove: true,
                Binds: binds,
                PortBindings: { '80/tcp': [{ HostPort: '80' }] },
                ...legoContainerOptions.HostConfig,
              },
            });

            startedContainers.addContainer(containerName);

            // eslint-disable-next-line no-param-reassign
            task.output = `Running lego ${command}...`;

            // The container is running, so a request may have been made - but a
            // result nobody read is not a result that can be reported.
            let result;
            try {
              result = await container.wait();
            } catch (e) {
              throw new LegoResultNotObservedError(e);
            }

            if (result.StatusCode !== 0) {
              // lego's own output is the best account of what went wrong -
              // Boulder answers "why did port 80 fail" in prose better than any
              // classifier dashmate could keep current.
              let errorMessage = `Lego exited with code ${result.StatusCode}`;
              try {
                const logs = await container.logs({
                  stdout: true,
                  stderr: true,
                });
                errorMessage += `\n${logs.toString()}`;
              } catch (e) {
                // Container may have been auto-removed
              }

              throw new Error(`Failed to obtain Let's Encrypt certificate: ${errorMessage}`);
            }

            // The authority has issued by this point, so the issuance counts
            // against this node's weekly limit however the rest of this run
            // goes. Recorded before anything else can fail, so a later problem
            // cannot hide it.
            ctx.certificateObtained = true;

            // Verify certificate and key were created
            if (!fs.existsSync(ctx.legoCertPath)) {
              throw new LegoArtifactsMissingError(ctx.legoCertPath);
            }

            if (!fs.existsSync(ctx.legoKeyPath)) {
              throw new LegoArtifactsMissingError(ctx.legoKeyPath);
            }
          };

          for (let attempt = 1; attempt <= MAX_OBTAIN_ATTEMPTS; attempt += 1) {
            try {
              await runLego();

              break;
            } catch (e) {
              // The helper never ran, so there is nothing the authority could
              // tell us and nothing to retry against - the fix is local.
              if (e instanceof LegoDidNotStartError) {
                throw new Error(renderHelperDidNotStartGuidance(config, e.cause));
              }

              if (e instanceof LegoResultNotObservedError) {
                throw new Error(renderResultNotObservedGuidance(config, e.cause));
              }

              // A certificate exists. Retrying would ask for another one for a
              // problem that is entirely local to this machine.
              if (e instanceof LegoArtifactsMissingError) {
                throw new Error(renderArtifactsMissingGuidance(config, e.missingPath));
              }

              // Prompting needs a positive opt-in from the entry point. The
              // helper renews inside a container with no terminal, where a
              // prompt would never settle and would hold the config lock -
              // and its event loop never drains, so it would hang forever.
              const canRetry = attempt < MAX_OBTAIN_ATTEMPTS
                && ctx.noRetry !== true
                && ctx.interactive === true;

              // Default No: an immediate retry cannot succeed, because the
              // operator has not left the terminal to change a firewall rule,
              // and each attempt spends one of the five failed authorizations
              // per hour this node shares with its own automatic renewal.
              const retry = canRetry && await promptOrThrow(task, {
                type: 'toggle',
                header: `  Let's Encrypt did not issue a certificate for ${ctx.externalIp}:

  ${e.message}

  Whatever the output above says is the reason - most often inbound port 80,
  but a rate limit or an account problem looks different and is not fixed by
  opening a firewall. Retrying without changing anything will fail the same
  way, so read it first, then answer Yes once something has changed.`,
                message: `Try again? [attempt ${attempt + 1} of ${MAX_OBTAIN_ATTEMPTS}]`,
                enabled: 'Yes',
                disabled: 'No',
                initial: false,
              }, { interactive: ctx.interactive });

              if (!retry) {
                throw new Error(`${e.message}\n\n${renderGiveUpGuidance(config, attempt)}`);
              }
            }
          }

          ctx.configurationUpdateRequired = true;

          // eslint-disable-next-line no-param-reassign
          task.output = 'Certificate obtained successfully';
        },
      },
      {
        title: 'Save certificate',
        skip: (ctx) => ctx.certificateValid && ctx.isCertificatePairInstalled,
        task: async (ctx) => {
          // Read certificate and key from lego output
          ctx.certificateFile = fs.readFileSync(ctx.legoCertPath, 'utf8');
          ctx.privateKeyFile = fs.readFileSync(ctx.legoKeyPath, 'utf8');
          ctx.configurationUpdateRequired = true;

          // Recorded here rather than after the issuance, so installing a
          // certificate that was already issued - a run recovering from an
          // interrupted one - counts as the gateway's certificate changing.
          ctx.certificateObtained = true;

          // Save to gateway SSL directory
          return saveCertificateTask(config);
        },
      },
      {
        title: 'Update configuration',
        enabled: (ctx) => ctx.configurationUpdateRequired,
        task: async () => {
          config.set('platform.gateway.ssl.enabled', true);
          config.set('platform.gateway.ssl.provider', 'letsencrypt');
        },
      },
    ], {
      rendererOptions: {
        showErrorMessage: true,
      },
    });
  }

  return obtainLetsEncryptCertificateTask;
}

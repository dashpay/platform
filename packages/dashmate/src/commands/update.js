import { Flags } from '@oclif/core';
import chalk from 'chalk';
import { Listr } from 'listr2';

import { NETWORK_MAINNET, NETWORK_TESTNET, OUTPUT_FORMATS } from '../constants.js';
import ConfigBaseCommand from '../oclif/command/ConfigBaseCommand.js';
import MuteOneLineError from '../oclif/errors/MuteOneLineError.js';
import printArrayOfObjects from '../printers/printArrayOfObjects.js';
import CertificateUnresolvedError from '../ssl/errors/CertificateUnresolvedError.js';
import { CERTIFICATE_STATUS } from '../ssl/checkGatewayCertificateFactory.js';
import renderCertificateGuidance from '../ssl/renderCertificateGuidance.js';
import isInteractiveSession from '../util/isInteractiveSession.js';

/**
 * Networks whose certificate has to be publicly trusted. Local and devnet nodes
 * are self-signed by design and disposable.
 */
const GATED_NETWORKS = [NETWORK_MAINNET, NETWORK_TESTNET];

/**
 * @param {string|undefined} value
 * @return {boolean}
 */
function isEnvironmentFlagSet(value) {
  if (value === undefined || value === null) {
    return false;
  }

  const normalized = String(value).trim().toLowerCase();

  return normalized !== '' && normalized !== '0' && normalized !== 'false';
}

export default class UpdateCommand extends ConfigBaseCommand {
  // The certificate check can obtain a certificate and record the provider that
  // issued it, so it holds the configuration lock for its whole run.
  static mutatesConfig = true;

  /**
   * The read-only preflight changes nothing, and it exists to be run before the
   * node is stopped - possibly while the helper is renewing. Taking a write
   * lock there would let it fail on a lock timeout for no reason.
   *
   * @param {Object} flags
   * @return {boolean}
   */
  static shouldSkipConfigLock(flags) {
    return flags['check-certificate'] === true;
  }

  static description = 'Update node software';

  static flags = {
    ...ConfigBaseCommand.flags,
    format: Flags.string({
      description: 'display output format',
      default: OUTPUT_FORMATS.PLAIN,
      options: Object.values(OUTPUT_FORMATS),
    }),
    'skip-certificate-check': Flags.boolean({
      description: 'do not act on the gateway certificate check. It still runs and still reports,'
        + ' but nothing is prompted, obtained or blocked. Also DASHMATE_SKIP_CERTIFICATE_CHECK',
      default: false,
    }),
    'non-interactive': Flags.boolean({
      description: 'never prompt. The certificate is checked and reported, nothing is obtained or'
        + ' changed. Also DASHMATE_NON_INTERACTIVE. Use CI=0 to prompt on a machine that exports CI',
      default: false,
    }),
    'check-certificate': Flags.boolean({
      description: 'only report on the gateway certificate and exit. Pulls no images, prompts for'
        + ' nothing and changes nothing. Safe to run before dashmate stop',
      default: false,
    }),
  };

  /**
   * @param {Object} args
   * @param {Object} flags
   * @param {docker} docker
   * @param {Config} config
   * @param {updateNode} updateNode
   * @param {checkGatewayCertificate} checkGatewayCertificate
   * @param {gatewayCertificateTask} gatewayCertificateTask
   * @param {DockerCompose} dockerCompose
   * @return {Promise<void>}
   */
  async runWithDependencies(
    args,
    flags,
    docker,
    config,
    updateNode,
    checkGatewayCertificate,
    gatewayCertificateTask,
    dockerCompose,
  ) {
    const {
      format,
      verbose: isVerbose,
      'check-certificate': checkCertificateOnly,
    } = flags;

    const skipCertificateCheck = flags['skip-certificate-check'] === true
      || isEnvironmentFlagSet(process.env.DASHMATE_SKIP_CERTIFICATE_CHECK);

    const interactive = isInteractiveSession({ flags });

    const isGated = config.get('platform.enable') === true
      && GATED_NETWORKS.includes(config.get('network'));

    /**
     * @param {Object} verdict
     * @return {Promise<void>}
     */
    const reportUnresolved = async (verdict) => {
      let isNodeRunning = false;
      try {
        isNodeRunning = await dockerCompose.isServiceRunning(config, 'gateway');
      } catch {
        // Docker being unavailable says nothing about the certificate, and the
        // node-state line is a courtesy rather than part of the verdict.
      }

      process.stderr.write(renderCertificateGuidance({
        config,
        verdict,
        isNodeRunning,
        pull: this.pullResult ?? null,
      }));
    };

    // Reports only. No pull is started, nothing is prompted, obtained, written
    // or reloaded - this is what an operator can run before stopping the node.
    if (checkCertificateOnly) {
      if (!isGated) {
        return;
      }

      const verdict = checkGatewayCertificate(config);

      process.stderr.write(`${JSON.stringify({
        status: verdict.status,
        reasons: verdict.reasons.map(({ code }) => code),
        warnings: verdict.warnings.map(({ code }) => code),
        provider: verdict.provider,
        config: config.getName(),
        expiresAt: verdict.installed ? verdict.installed.validTo.toISOString() : null,
      })}\n`);

      if (verdict.status === CERTIFICATE_STATUS.INVALID) {
        await reportUnresolved(verdict);

        throw new MuteOneLineError(new CertificateUnresolvedError(verdict));
      }

      return;
    }

    // A prompt that leaks past the interactivity guard neither throws nor
    // settles: the event loop simply drains and the process exits 0 with
    // nothing done. Failing closed here turns that silence into an exit code.
    process.exitCode = 1;

    // Both handlers are attached the moment the pull is created, so the minutes
    // the certificate task may spend at a prompt are not a window in which an
    // unhandled rejection can take the process down. updateNode is async and
    // calls getServiceList synchronously, and docker.pull can throw
    // synchronously inside its executor, so this promise really can reject.
    const settled = updateNode(config).then(
      (info) => ({ ok: true, info }),
      (error) => ({ ok: false, error }),
    );

    let pullReported = false;

    const reportPull = async () => {
      if (pullReported) {
        return;
      }
      pullReported = true;

      const result = await settled;

      if (!result.ok) {
        this.pullResult = { ok: false, failed: 0, total: 0 };

        process.stderr.write(`Failed to pull images: ${result.error.message}\n`);

        return;
      }

      this.pullResult = {
        ok: true,
        failed: result.info.filter(({ updated }) => updated === 'error').length,
        total: result.info.length,
      };

      const colors = {
        updated: chalk.yellow,
        'up to date': chalk.green,
        error: chalk.red,
      };

      printArrayOfObjects(result.info.map(({
        name, title, updated, image,
      }) => (format === OUTPUT_FORMATS.PLAIN
        ? { Service: title, Image: image, Updated: colors[updated](updated) }
        : {
          name, title, updated, image,
        })), format);
    };

    const tasks = new Listr(
      [
        {
          title: 'Gateway certificate',
          enabled: () => isGated,
          task: gatewayCertificateTask(config, { interactive, skipCertificateCheck }),
        },
        {
          title: 'Update node software',
          task: () => reportPull(),
        },
      ],
      {
        // The certificate task signals an unresolved certificate by throwing,
        // because throwing is the only way to render a listr2 task as failed.
        // Without this the throw would skip the pull report entirely, hiding
        // the table - including any image that failed to download.
        exitOnError: false,
        // Interactivity beats --verbose: the verbose renderer manages no prompt
        // area, and -v is exactly what an operator adds when the check has just
        // failed.
        renderer: (format === OUTPUT_FORMATS.JSON && 'silent')
          || (interactive && 'default')
          || (isVerbose && 'verbose')
          || 'default',
        rendererOptions: {
          showTimer: isVerbose,
          clearOutput: false,
          collapse: false,
          showSubtasks: true,
          removeEmptyLines: false,
        },
      },
    );

    const context = {};

    try {
      await tasks.run(context);
    } finally {
      // Covers what task 2 cannot: an exception from run() itself, or from the
      // reporting path. The guard makes the second call harmless, and the table
      // is rendered before any failure is reported either way.
      await reportPull();
    }

    // listr2 wraps what a task threw, so the sentinel is one level down.
    const errors = (tasks.err ?? []).map((error) => error?.error ?? error);
    const unresolved = errors.find((error) => error instanceof CertificateUnresolvedError);
    const unexpected = errors.find((error) => !(error instanceof CertificateUnresolvedError));

    // Under JSON output stdout is exactly one parseable array, so everything a
    // machine might want about the certificate goes to stderr as one line.
    if (format === OUTPUT_FORMATS.JSON && context.certificate) {
      process.stderr.write(`${JSON.stringify({
        status: context.certificate.status,
        reasons: context.certificate.reasons.map(({ code }) => code),
        warnings: context.certificate.warnings.map(({ code }) => code),
        provider: context.certificate.provider,
        config: config.getName(),
        expiresAt: context.certificate.installed
          ? context.certificate.installed.validTo.toISOString()
          : null,
        skipped: context.certificateSkipped === true,
        pull: this.pullResult ?? null,
      })}\n`);
    }

    (context.certificateWarnings ?? []).forEach((warning) => {
      process.stderr.write(`${warning}\n\n`);
    });

    if (context.certificateSkipped) {
      process.stderr.write(`Gateway certificate enforcement was skipped.`
        + ` The check still ran and its status is ${context.certificate.status}.\n\n`);
    }

    if (context.certificateSuccess) {
      process.stderr.write(`${context.certificateSuccess}\n`);
    }

    // A lost lock, a failed reload or a programming error is a real failure and
    // must not be reduced to a certificate message. exitOnError would otherwise
    // have swallowed it.
    if (unexpected) {
      throw unexpected;
    }

    if (unresolved) {
      await reportUnresolved(unresolved.getVerdict());

      throw new MuteOneLineError(unresolved);
    }

    process.exitCode = 0;
  }
}

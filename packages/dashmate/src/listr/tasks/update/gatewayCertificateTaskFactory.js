import { SSL_PROVIDERS } from '../../../constants.js';
import ServiceIsNotRunningError from '../../../docker/errors/ServiceIsNotRunningError.js';
import CertificateUnresolvedError from '../../../ssl/errors/CertificateUnresolvedError.js';
import {
  CERTIFICATE_REASONS,
  CERTIFICATE_STATUS,
} from '../../../ssl/checkGatewayCertificateFactory.js';
import promptOrThrow from '../../../util/promptOrThrow.js';
import renderConfigFlag from '../../../util/renderConfigFlag.js';

/**
 * Below this the switch is offered with Yes preselected. Six of the twenty-one
 * ZeroSSL certificates still alive on mainnet at the time of the census had a
 * week or less left.
 */
const ZEROSSL_URGENT_DAYS = 14;

/**
 * What declining actually leaves behind, said only as far as the verdict goes.
 *
 * Always the pair installed for the gateway, never what the node is serving:
 * these checks read files and never opened a connection, so the two are not
 * known to be the same thing. And never that nothing is wrong in general - only
 * that nothing stopped this update.
 *
 * @param {Object} verdict
 * @return {string}
 */
function renderDeclining(verdict) {
  if (verdict.status === CERTIFICATE_STATUS.CHECKS_PASSED) {
    return `  The certificate installed for the gateway passed these checks and stays
  in place, so nothing changes if you decline.
`;
  }

  if (verdict.status !== CERTIFICATE_STATUS.WARN) {
    return `  Declining leaves the certificate installed for the gateway exactly as it
  is: unchanged, and still failing the checks above.
`;
  }

  // Rendered here rather than referred to. This prompt is where the operator
  // decides, and the warnings are printed only once the command has finished,
  // so pointing at them would be pointing at something not yet on screen.
  return `  Declining leaves the certificate installed for the gateway exactly as it
  is. Nothing about it stopped this update, but these checks did find:

${verdict.warnings.map(({ message }) => `    - ${message}\n`).join('')}`;
}

/**
 * The whole argument for switching, including the port-80 requirement, in the
 * prompt header - this is the last moment the operator can go and open a
 * firewall rule instead of failing three times.
 *
 * @param {Config} config
 * @param {string} externalIp
 * @param {Object} [options]
 * @param {Object} [options.verdict] - decides what declining leaves behind, and
 *   carries the warnings the operator is being asked to weigh
 * @return {string}
 */
function renderSwitchOffer(config, externalIp, { verdict } = {}) {
  return `  Switching this node to Let's Encrypt will:
    - obtain a new certificate now, free, for ${externalIp}
    - change platform.gateway.ssl.provider from ${config.get('platform.gateway.ssl.provider')} to letsencrypt
    - leave your existing provider's account and credentials untouched but unused

  It needs inbound port 80 reachable from the internet right now, and it needs
  port 80 permanently thereafter - not on a schedule you can plan around.
  Certificates for IP addresses last about six days and dashmate renews them
  continuously for as long as this node runs. A rule you open now and close
  later, or one that does not survive a reboot, takes this node dark within
  six days.

  It usually takes under a minute. If port 80 is not open yet, answer No, open
  it permanently, and re-run dashmate update ${renderConfigFlag(config.getName())}.

  Your image pull is running now and will finish either way, so answering No
  does not hold this node back from protocol upgrades or security patches.
${renderDeclining(verdict ?? { status: CERTIFICATE_STATUS.INVALID, warnings: [] })}`;
}

/**
 * The operator who just succeeded is the one who never sees a port-80 failure
 * message, and is quite possibly the one who opened port 80 by hand for this
 * migration alone. Saying nothing here reproduces the dark-node failure in a
 * fresh cohort within a week.
 *
 * @param {Config} config
 * @param {Object} verdict - the verdict taken after the obtain
 * @return {string}
 */
function renderSuccess(config, verdict) {
  const expiresAt = verdict.installed
    ? verdict.installed.validTo.toISOString().slice(0, 10)
    : 'unknown';
  const days = verdict.expiresInDays === null ? '?' : Math.floor(verdict.expiresInDays);

  return `  Certificate obtained from Let's Encrypt for ${config.get('externalIp')}
  Valid until ${expiresAt} (about ${days} days).

  LEAVE PORT 80 OPEN. This was not a one-time requirement. Certificates for
  IP addresses last about six days, and dashmate keeps renewing this one for
  as long as the node runs - every renewal needs inbound port 80 again.

  If you opened port 80 just now to make this work, make the rule permanent
  and make sure it survives a reboot. If it lapses, this node goes dark
  within six days.

  Nothing will warn you: Let's Encrypt stopped sending expiry emails on
  2025-06-04. Check with: dashmate doctor ${renderConfigFlag(config.getName())}
`;
}

/**
 * Carry a verdict's warnings into the run's report.
 *
 * The command prints only what is collected here, so a branch that re-checks
 * and returns without this drops any warning the new state carries - a
 * provider that still disagrees, or an accepted self-signed certificate on a
 * fullnode - from everything except machine output.
 *
 * @param {Object} ctx
 * @param {Object} verdict
 */
function collectWarnings(ctx, verdict) {
  if (verdict.warnings.length === 0) {
    return;
  }

  ctx.certificateWarnings = [
    ...(ctx.certificateWarnings ?? []),
    ...verdict.warnings.map(({ message }) => message),
  ];
}

/**
 * @param {Object} verdict
 * @param {string} code
 * @return {boolean}
 */
function hasReason(verdict, code) {
  return verdict.reasons.some((reason) => reason.code === code);
}

/**
 * @param {checkGatewayCertificate} checkGatewayCertificate
 * @param {obtainLetsEncryptCertificateTask} obtainLetsEncryptCertificateTask
 * @param {installCertificateFilesTask} installCertificateFilesTask
 * @param {ConfigFileJsonRepository} configFileRepository
 * @param {ConfigFile} configFile
 * @param {writeConfigTemplates} writeConfigTemplates
 * @param {DockerCompose} dockerCompose
 * @return {gatewayCertificateTask}
 */
export default function gatewayCertificateTaskFactory(
  checkGatewayCertificate,
  obtainLetsEncryptCertificateTask,
  installCertificateFilesTask,
  configFileRepository,
  configFile,
  writeConfigTemplates,
  dockerCompose,
) {
  /**
   * Persist the provider, and only after a certificate exists to back it.
   *
   * Writing the provider first and then failing to obtain converts a node that
   * was working with an expiring certificate into one that is broken:
   * configuration would name an authority the node has no account with, and
   * the helper's watcher would reschedule renewal against it within a minute,
   * forever.
   *
   * @param {Config} config
   */
  function persistProvider(config) {
    // Issuance takes minutes, which is long enough for this lease to be lost
    // and another command to save and render newer state. Rendering from this
    // configuration would overwrite that.
    if (!configFileRepository.isExclusive()) {
      throw new Error('Lost the configuration lock while obtaining the certificate, so the'
        + ' provider was not saved. The certificate was obtained and installed; re-run once no'
        + ' other command is changing configuration.');
    }

    // Written immediately rather than at command exit, because update goes on
    // into a multi-minute image pull afterwards. Both calls are needed: saving
    // clears the collection's changed flag but leaves the individual config
    // marked changed until its templates render.
    configFileRepository.write(configFile);
    writeConfigTemplates(config);
  }

  /**
   * Envoy reads the certificate files once at startup. Under the documented
   * upgrade procedure the node is stopped and the new certificate loads at the
   * next start, but update against a running node is supported too and there
   * the reload is what makes the change reach the wire.
   *
   * A signal is sufficient and nothing here needs to restart the container.
   * PID 1 in the gateway container is Envoy's hot-restarter, not Envoy: its
   * SIGHUP handler forks and re-execs Envoy with an incremented restart epoch
   * against the same envoy.yaml. The new process parses that file from scratch
   * and opens the certificate by name, so both a renewed certificate and a
   * changed listener structure take effect while the old process drains. A
   * container restart would achieve the same thing and cost an outage.
   *
   * @param {Config} config
   * @return {Promise<void>}
   */
  async function reloadGateway(config) {
    try {
      await dockerCompose.execCommand(config, 'gateway', 'kill -SIGHUP 1');
    } catch (e) {
      if (!(e instanceof ServiceIsNotRunningError)) {
        throw e;
      }
    }
  }

  /**
   * Check the certificate installed for the gateway and, when an operator is
   * there to answer, offer to repair it.
   *
   * @typedef {gatewayCertificateTask}
   * @param {Config} config
   * @param {Object} options
   * @param {boolean} options.interactive
   * @param {boolean} [options.skipCertificateCheck]
   * @return {function(Object, Object): Promise<void>}
   */
  function gatewayCertificateTask(config, { interactive, skipCertificateCheck = false }) {
    const cfg = renderConfigFlag(config.getName());

    /**
     * Run an obtain and decide what its outcome means.
     *
     * A failure is judged by re-checking the installed pair rather than by
     * where it happened. An obtain that failed before touching the gateway
     * files leaves the node exactly as it was; one that failed between the two
     * writes can have replaced a working pair with a mismatched one, and
     * reporting success there would tell an operator their node is fine at the
     * moment it stopped serving TLS.
     *
     * @param {Object} ctx
     * @param {Function} run
     * @return {Promise<Object>} the verdict after the attempt
     */
    async function attemptObtain(ctx, run) {
      try {
        await run();
      } catch (e) {
        ctx.certificateObtainError = e;

        return checkGatewayCertificate(config);
      }

      persistProvider(config);

      await reloadGateway(config);

      return checkGatewayCertificate(config);
    }

    /**
     * @param {Object} ctx
     * @return {Promise<Object>}
     */
    async function switchToLetsEncrypt(ctx) {
      return attemptObtain(ctx, () => obtainLetsEncryptCertificateTask(config)
        .run({ ...ctx, interactive }));
    }

    return async (ctx, task) => {
      const verdict = checkGatewayCertificate(config);

      ctx.certificate = verdict;

      // The check always runs, even when enforcement is bypassed, so a playbook
      // carrying the flag keeps surfacing the problem instead of muting it.
      if (skipCertificateCheck) {
        ctx.certificateSkipped = true;

        task.skip(`Enforcement skipped, status is ${verdict.status}`);

        return;
      }

      // Anything short of blocking, not only a spotless verdict. A ZeroSSL node
      // reaches WARN through any warning at all, including the certificate
      // running out inside a day - and that is when the switch below matters
      // most, so gating it on a clean verdict withheld the offer at exactly the
      // moment it was worth making.
      if (verdict.status !== CERTIFICATE_STATUS.INVALID) {
        // Someone who bought a certificate is never nagged. ZeroSSL is the one
        // exception, because a free account stops being able to renew and the
        // operator has no way to find that out until it has happened.
        if (verdict.provider !== SSL_PROVIDERS.ZEROSSL) {
          collectWarnings(ctx, verdict);

          return;
        }

        const daysLeft = Math.floor(verdict.expiresInDays ?? 0);
        // Below a day this floors to zero, and "expires in 0 days" reads as a
        // rendering fault rather than as the most urgent thing on the page.
        const remaining = daysLeft < 1 ? 'in less than a day' : `in ${daysLeft} days`;

        // Said on every run, to a human and to a script alike: a free ZeroSSL
        // account allows three certificates in total, and nothing tells an
        // operator that renewal has stopped being possible until it has.
        const warn = () => {
          ctx.certificateWarnings = [
            ...(ctx.certificateWarnings ?? []),
            `This node is configured to use ZeroSSL, and the certificate it has`
            + ` installed expires ${remaining}. A free ZeroSSL`
            + " account allows three certificates in total, so dashmate's renewals stop"
            + ` working after about 270 days. Switch to Let's Encrypt with:`
            + `\n    dashmate ssl obtain ${cfg} --provider letsencrypt`,
          ];
        };

        if (!interactive) {
          warn();
          collectWarnings(ctx, verdict);

          return;
        }

        const accepted = await promptOrThrow(task, {
          type: 'toggle',
          header: renderSwitchOffer(config, config.get('externalIp'), { verdict }),
          message: "Switch to Let's Encrypt and obtain a certificate now?",
          enabled: 'Yes',
          disabled: 'Not now',
          initial: daysLeft < ZEROSSL_URGENT_DAYS,
        }, { interactive });

        if (!accepted) {
          warn();
          collectWarnings(ctx, verdict);

          return;
        }

        const after = await switchToLetsEncrypt(ctx);

        // Nothing was blocking before this ran, so a failure that left the node
        // as it was is a warning. A failure that damaged the installed pair is
        // not, and this is the only thing that can tell them apart. Judged the
        // same way as every other branch - anything short of blocking is a
        // node that still works, and a certificate can cross the
        // expiring-soon boundary during a multi-minute failed obtain.
        if (after.status !== CERTIFICATE_STATUS.INVALID) {
          ctx.certificate = after;

          if (ctx.certificateObtainError) {
            // Nothing was touched, so the node still holds the ZeroSSL
            // certificate and still needs to hear about it.
            warn();

            ctx.certificateWarnings.push(
              `The switch to Let's Encrypt did not complete: ${ctx.certificateObtainError.message}`
              + '\nThe certificate this node was already using is untouched.',
            );

            collectWarnings(ctx, after);
          } else {
            // The node is no longer on ZeroSSL, so its expiry is no longer
            // this node's problem and repeating it would contradict the
            // success message.
            ctx.certificateSuccess = renderSuccess(config, after);
          }

          return;
        }

        ctx.certificate = after;

        throw new CertificateUnresolvedError(after);
      }

      // INVALID from here on. Nothing is acted on without an operator: a
      // configuration change nobody asked for, made unattended on infrastructure
      // they own, is not dashmate's to make - and it would replace a diagnosis
      // with a silent failure.
      if (!interactive) {
        throw new CertificateUnresolvedError(verdict);
      }

      // Only when the interrupted switch is the whole problem. The pair being
      // byte-identical to the one lego produced says nothing about whether it
      // is still valid, so this state can carry an expired or misaddressed
      // certificate alongside it - and there the setting is not all that is
      // missing. Those fall through to the obtain below.
      if (verdict.reasons.length === 1
        && hasReason(verdict, CERTIFICATE_REASONS.SWITCH_INCOMPLETE)) {
        const complete = await promptOrThrow(task, {
          type: 'toggle',
          header: `  A Let's Encrypt certificate is installed for the gateway, but the
  configuration still names ${verdict.provider}, so dashmate's helper is renewing the
  wrong provider. No certificate needs to be obtained - only the setting has
  to be saved.\n`,
          message: 'Finish the interrupted switch now?',
          enabled: 'Yes',
          disabled: 'No',
          initial: true,
        }, { interactive });

        if (!complete) {
          throw new CertificateUnresolvedError(verdict);
        }

        config.set('platform.gateway.ssl.enabled', true);
        config.set('platform.gateway.ssl.provider', SSL_PROVIDERS.LETSENCRYPT);

        persistProvider(config);

        await reloadGateway(config);

        // Judged by what the node holds afterwards, like every other branch.
        // Persisting can fail, the reload can fail, and the installed pair can
        // expire between the check and the write - telling an operator their
        // node is fixed when it is dark is worse than saying nothing.
        const after = checkGatewayCertificate(config);

        ctx.certificate = after;
        collectWarnings(ctx, after);

        if (after.status === CERTIFICATE_STATUS.INVALID) {
          throw new CertificateUnresolvedError(after);
        }

        return;
      }

      // An operator with their own certificate is offered the chance to replace
      // it before changing authority is even suggested.
      if ([SSL_PROVIDERS.FILE, SSL_PROVIDERS.SELF_SIGNED].includes(verdict.provider)) {
        const installFiles = await promptOrThrow(task, {
          type: 'toggle',
          header: `  ${verdict.reasons[0].message}

  If you have a replacement certificate and key on disk, dashmate can install
  them for the gateway now.\n`,
          message: 'Install new certificate files now?',
          enabled: 'Yes',
          disabled: 'No',
          initial: verdict.provider === SSL_PROVIDERS.FILE,
        }, { interactive });

        if (installFiles) {
          await installCertificateFilesTask(config, { interactive }).run({ ...ctx, interactive });

          // The gateway listener is branched on the provider: self-signed
          // renders a tls_inspector and a raw_buffer filter chain, so the port
          // goes on accepting plaintext connections. Leaving the setting behind
          // would keep that chain on a node that now holds a real certificate.
          //
          // Written only now, after the files are installed, so configuration
          // can never name a provider the node has no certificate for. Saved
          // immediately rather than at command exit, because update carries on
          // into a multi-minute pull and the end-of-run save is skipped
          // whenever the run later throws.
          config.set('platform.gateway.ssl.enabled', true);
          config.set('platform.gateway.ssl.provider', SSL_PROVIDERS.FILE);

          persistProvider(config);

          await reloadGateway(config);

          const after = checkGatewayCertificate(config);
          ctx.certificate = after;
          collectWarnings(ctx, after);

          if (after.status !== CERTIFICATE_STATUS.INVALID) {
            return;
          }

          throw new CertificateUnresolvedError(after);
        }
      }

      // Let's Encrypt is already configured, so there is nothing to switch to -
      // it is the only authority that issues IP-address certificates over ACME.
      // The offer is to try obtaining again.
      const isAlreadyLetsEncrypt = verdict.provider === SSL_PROVIDERS.LETSENCRYPT;

      const header = isAlreadyLetsEncrypt
        ? `  ${verdict.reasons[0].message}

  This node is already on Let's Encrypt, so there is no provider to switch to.
  Renewal has most likely been failing without anyone being told; dashmate has
  not inspected the helper's history to confirm that.

  The most likely cause is inbound port 80, which Let's Encrypt re-checks on
  every renewal - permanently, roughly every four days. It is not always port
  80: half the nodes in this state have it open and stopped renewing anyway.\n`
        : renderSwitchOffer(config, config.get('externalIp'));

      const accepted = await promptOrThrow(task, {
        type: 'toggle',
        header,
        message: isAlreadyLetsEncrypt
          ? 'Try to obtain a new certificate now?'
          : "Switch to Let's Encrypt and obtain a certificate now?",
        enabled: 'Yes',
        disabled: 'No',
        // Nothing works today, so trying costs the operator nothing they still
        // have. The exception is a certificate they bought, where changing
        // authority is a decision only they can make.
        initial: verdict.provider !== SSL_PROVIDERS.FILE,
      }, { interactive });

      if (!accepted) {
        throw new CertificateUnresolvedError(verdict);
      }

      const after = await switchToLetsEncrypt(ctx);

      ctx.certificate = after;

      if (after.status === CERTIFICATE_STATUS.INVALID) {
        throw new CertificateUnresolvedError(after);
      }

      ctx.certificateSuccess = renderSuccess(config, after);
    };
  }

  return gatewayCertificateTask;
}

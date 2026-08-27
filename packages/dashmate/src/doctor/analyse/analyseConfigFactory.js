import chalk from 'chalk';
import { NETWORK_LOCAL, NETWORK_MAINNET } from '../../constants.js';
import { ERRORS as LETSENCRYPT_ERRORS } from '../../ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';
import { ERRORS as ZEROSSL_ERRORS } from '../../ssl/zerossl/validateZeroSslCertificateFactory.js';
import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';
import renderConfigFlag from '../../util/renderConfigFlag.js';

/**
 * Whether a ZeroSSL certificate can be renewed depends on the operator's plan, which dashmate
 * cannot see, so both routes are offered rather than assuming which one applies.
 */
const LETSENCRYPT_ALTERNATIVE = chalk`Or switch to Let's Encrypt, which issues certificates for IP addresses free
of charge:
  {bold.cyanBright dashmate config set platform.gateway.ssl.provider letsencrypt}
  {bold.cyanBright dashmate config set platform.gateway.ssl.providerConfigs.letsencrypt.email EMAIL}
  {bold.cyanBright dashmate ssl obtain}`;

export default function analyseConfigFactory() {
  /**
   * @typedef analyseConfig
   * @param {Samples} samples
   * @return {Problem[]}
   */
  function analyseConfig(samples) {
    const config = samples.getDashmateConfig();

    const problems = [];

    if (config?.get('platform.enable')) {
      // Platform Node ID
      const masternodeStatus = samples.getServiceInfo('core', 'masternodeStatus');
      const platformNodeId = masternodeStatus?.dmnState?.platformNodeId;
      if (platformNodeId && config.get('platform.drive.tenderdash.node.id') !== platformNodeId) {
        const problem = new Problem(
          'Platform Node ID doesn\'t match the one found in the ProReg transaction',
          chalk`Please set the correct Node ID and Node Key:
  {bold.cyanBright dashmate config set platform.drive.tenderdash.node.id ID
  dashmate config set platform.drive.tenderdash.node.key KEY}
  Or update the Node ID in the masternode list using a ProServUp transaction`,
          SEVERITY.HIGH,
        );

        problems.push(problem);
      }

      // SSL certificate
      const ssl = samples.getServiceInfo('gateway', 'ssl');
      if (ssl?.error) {
        switch (ssl.error) {
          case 'disabled':
            if (config.get('network') !== NETWORK_LOCAL) {
              const problem = new Problem(
                'SSL certificates are disabled. Clients won\'t be able to connect securely',
                chalk`Please enable and set up SSL certificates {bold.cyanBright https://docs.dash.org/en/stable/docs/user/masternodes/setup-evonode.html#ssl-certificates}`,
                SEVERITY.HIGH,
              );

              problems.push(problem);
            }
            break;
          case 'self-signed':
            if (config.get('network') === NETWORK_MAINNET) {
              const problem = new Problem(
                'Self-signed SSL certificate is used on mainnet. Clients won\'t be able to connect securely',
                chalk`Please use valid SSL certificates {bold.cyanBright https://docs.dash.org/en/stable/docs/user/masternodes/setup-evonode.html#ssl-certificates}`,
                SEVERITY.HIGH,
              );

              problems.push(problem);
            }
            break;
          default: {
            const fileProblems = {
              // File provider error
              'not-valid': {
                description: 'SSL certificate files are not valid',
                solution: chalk`Please make sure the certificate chain contains the actual server certificate at the top of the file, and it corresponds to the private key

Certificate chain file path: {bold.cyanBright ${ssl?.data?.chainFilePath}}
Private key file path: {bold.cyanBright ${ssl?.data?.privateFilePath}}`,
              },
              // File provider error
              'not-exist': {
                description: 'SSL certificate files are not found',
                solution: chalk`Please get an SSL certificate and place the certificate files in the correct location.

Certificate chain file path: {bold.cyanBright ${ssl?.data?.chainFilePath}}
Private key file path: {bold.cyanBright ${ssl?.data?.privateFilePath}}

Or use ZeroSSL https://docs.dash.org/en/stable/docs/user/masternodes/setup-evonode.html#ssl-certificates`,
              },
            };

            const zeroSslProblems = {
              [ZEROSSL_ERRORS.API_KEY_IS_NOT_SET]: {
                description: 'ZeroSSL API key is not set.',
                solution: chalk`Please obtain your API key from {underline.cyanBright https://app.zerossl.com/developer}
And then update your configuration with {bold.cyanBright dashmate config set platform.gateway.ssl.providerConfigs.zerossl.apiKey [KEY]}`,
              },
              [ZEROSSL_ERRORS.EXTERNAL_IP_IS_NOT_SET]: {
                description: 'External IP is not set.',
                solution: chalk`Please update your configuration to include your external IP using {bold.cyanBright dashmate config set externalIp [IP]}`,
              },
              [ZEROSSL_ERRORS.CERTIFICATE_ID_IS_NOT_SET]: {
                description: 'ZeroSSL certificate is not configured',
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain} to get a new certificate`,
              },
              [ZEROSSL_ERRORS.PRIVATE_KEY_IS_NOT_PRESENT]: {
                description: chalk`ZeroSSL private key file not found in ${ssl?.data?.privateKeyFilePath}.`,
                solution: chalk`Please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --force}
and revoke the previous certificate in the ZeroSSL dashboard`,
              },
              [ZEROSSL_ERRORS.EXTERNAL_IP_MISMATCH]: {
                description: chalk`ZeroSSL IP ${ssl?.data?.certificate?.common_name} does not match external IP ${ssl?.data?.externalIp}.`,
                solution: chalk`Please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --force}
            and revoke the previous certificate in the ZeroSSL dashboard`,
              },
              [ZEROSSL_ERRORS.CSR_FILE_IS_NOT_PRESENT]: {
                description: chalk`ZeroSSL certificate request file not found in ${ssl?.data?.csrFilePath}.
This makes auto-renewal impossible.`,
                solution: chalk`If you need auto renew, please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --force}
and revoke the previous certificate in the ZeroSSL dashboard`,
              },
              [ZEROSSL_ERRORS.CERTIFICATE_EXPIRES_SOON]: {
                description: chalk`ZeroSSL certificate expires at ${ssl?.data?.certificate?.expires}.`,
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain} to get a new one, which needs an
available certificate on your ZeroSSL plan.

${LETSENCRYPT_ALTERNATIVE}`,
              },
              [ZEROSSL_ERRORS.CERTIFICATE_IS_NOT_VALIDATED]: {
                description: chalk`ZeroSSL certificate is not approved.`,
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain} to confirm certificate`,
              },
              [ZEROSSL_ERRORS.CERTIFICATE_IS_NOT_VALID]: {
                description: chalk`ZeroSSL certificate is not valid.`,
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain} to get a new one.

${LETSENCRYPT_ALTERNATIVE}`,
              },
              [ZEROSSL_ERRORS.ZERO_SSL_API_ERROR]: {
                // ZeroSSL's own wording is the most accurate account of what went wrong - it
                // names an exhausted certificate limit, an unpaid invoice or a rejected key
                // directly. The fallback keeps the problem reported when it sends none, since
                // an empty description would otherwise drop it silently.
                description: ssl?.data?.error?.message
                  ? chalk`ZeroSSL rejected the request: ${ssl.data.error.message}`
                  : chalk`The ZeroSSL API could not be reached, so the certificate cannot be checked or renewed.`,
                solution: chalk`If this is something you can resolve with ZeroSSL, such as an expired plan or a
rejected API key, fix it there and run {bold.cyanBright dashmate ssl obtain}.

${LETSENCRYPT_ALTERNATIVE}`,
              },
            };

            const letsEncryptProblems = {
              [LETSENCRYPT_ERRORS.EXTERNAL_IP_IS_NOT_SET]: {
                description: 'External IP is not set.',
                solution: chalk`Please update your configuration to include your external IP using {bold.cyanBright dashmate config set externalIp [IP]}`,
              },
              [LETSENCRYPT_ERRORS.CERTIFICATE_NOT_FOUND]: {
                description: 'Let\'s Encrypt certificate is not configured',
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain --provider=letsencrypt} to get a new certificate`,
              },
              [LETSENCRYPT_ERRORS.PRIVATE_KEY_NOT_FOUND]: {
                description: chalk`Let's Encrypt private key file not found.`,
                solution: chalk`Please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --provider=letsencrypt --force}`,
              },
              [LETSENCRYPT_ERRORS.CERTIFICATE_IP_MISMATCH]: {
                description: chalk`Let's Encrypt certificate does not match external IP ${ssl?.data?.externalIp}.`,
                solution: chalk`Please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --provider=letsencrypt --force}`,
              },
              [LETSENCRYPT_ERRORS.CERTIFICATE_EXPIRES_SOON]: {
                description: chalk`Let's Encrypt certificate expires at ${ssl?.data?.certificate?.expires}.`,
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain --provider=letsencrypt} to renew`,
              },
              // Never a restart. This fires because the issued certificate was not
              // copied to where the gateway loads from, so a restart makes the gateway
              // re-read the copy it already has - the out-of-date one. On a node still
              // serving a valid certificate that is what takes it off the network.
              [LETSENCRYPT_ERRORS.CERTIFICATE_NOT_INSTALLED]: {
                description: chalk`A renewed Let's Encrypt certificate has not been installed for the gateway.`,
                solution: chalk`The issued certificate was never copied to where the gateway loads
from. Install it - no restart needed:
{bold.cyanBright dashmate ssl obtain ${renderConfigFlag(config.getName())} --provider=letsencrypt}

Do not restart Platform. That reloads the out-of-date copy and may throw away
a working certificate.`,
              },
              [LETSENCRYPT_ERRORS.CERTIFICATE_NOT_VALID]: {
                description: chalk`Let's Encrypt certificate is not valid.`,
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain --provider=letsencrypt --force} to get a new one.`,
              },
            };

            // Both providers report some errors under the same name, so only the
            // configured provider's messages are considered. Otherwise one provider's
            // message would describe a problem found by the other one.
            const providerProblems = config.get('platform.gateway.ssl.provider') === 'letsencrypt'
              ? letsEncryptProblems
              : zeroSslProblems;

            const {
              description,
              solution,
              severity = SEVERITY.HIGH,
            } = {
              ...fileProblems,
              ...providerProblems,
            }[ssl.error] ?? {};

            if (description) {
              // These predate the renewal record and each one ends in its own
              // request. A node that cannot write a certificate down would be
              // handed one here regardless of what every other surface decided
              // - and for ZeroSSL the allowance is three in a node's lifetime,
              // not five a week.
              const storageRefuses = samples
                .getServiceInfo('gateway', 'certificateRenewal')?.storageWritable === false;

              const problem = new Problem(
                description,
                storageRefuses && solution?.includes('ssl obtain')
                  ? chalk`Free disk space and check permissions on this node's certificate
directory first. A certificate obtained now could not be saved, and each
one counts against this node's certificate allowance.`
                  : solution,
                severity,
              );

              problems.push(problem);
            }
            break;
          }
        }
      }

      if (samples?.getDashmateConfig()?.get('network') !== NETWORK_LOCAL) {
        // Core P2P port
        const coreP2pPort = samples.getServiceInfo('core', 'p2pPort');
        if (coreP2pPort && coreP2pPort !== 'OPEN') {
          const port = config.get('core.p2p.port');
          const externalIp = config.get('externalIp');

          let solution = chalk`Please ensure that port ${port} on your public IP address ${externalIp} is open
for incoming connections. You may need to configure your firewall to
ensure this port is accessible from the public internet. If you are using
Network Address Translation (NAT), please enable port forwarding for port 80
and all Dash service ports listed above.`;
          if (externalIp) {
            solution = chalk`Please ensure your configured IP address ${externalIp} is your public IP.
You can change it using {bold.cyanBright dashmate config set externalIp [IP]}.
Also, ensure that port ${port} on your public IP address is open
for incoming connections. You may need to configure your firewall to
ensure this port is accessible from the public internet. If you are using
Network Address Translation (NAT), please enable port forwarding for port 80
and all Dash service ports listed above.`;
          }

          const problem = new Problem(
            'Core P2P port is unavailable for incoming connections.',
            solution,
            SEVERITY.HIGH,
          );

          problems.push(problem);
        }

        // Gateway HTTP port
        const gatewayHttpPort = samples.getServiceInfo('gateway', 'httpPort');
        if (gatewayHttpPort && gatewayHttpPort !== 'OPEN') {
          const port = config.get('platform.gateway.listeners.dapiAndDrive.port');
          const externalIp = config.get('externalIp');

          let solution = chalk`Please ensure that port ${port} on your public IP address ${externalIp} is open
for incoming connections. You may need to configure your firewall to
ensure this port is accessible from the public internet. If you are using
Network Address Translation (NAT), please enable port forwarding for port 80
and all Dash service ports listed above.`;
          if (externalIp) {
            solution = chalk`Please ensure your configured IP address ${externalIp} is your public IP.
You can change it using {bold.cyanBright dashmate config set externalIp [IP]}.
Also, ensure that port ${port} on your public IP address is open
for incoming connections. You may need to configure your firewall to
ensure this port is accessible from the public internet. If you are using
Network Address Translation (NAT), please enable port forwarding for port 80
and all Dash service ports listed above.`;
          }

          const problem = new Problem(
            'Gateway HTTP port is unavailable for incoming connections.',
            solution,
            SEVERITY.HIGH,
          );

          problems.push(problem);
        }

        // Tenderdash P2P port
        const tenderdashP2pPort = samples.getServiceInfo('drive_tenderdash', 'p2pPort');
        if (tenderdashP2pPort && tenderdashP2pPort !== 'OPEN') {
          const port = config.get('platform.drive.tenderdash.p2p.port');
          const externalIp = config.get('externalIp');

          let solution = chalk`Please ensure that port ${port} on your public IP address ${externalIp} is open
for incoming connections. You may need to configure your firewall to
ensure this port is accessible from the public internet. If you are using
Network Address Translation (NAT), please enable port forwarding for port 80
and all Dash service ports listed above.`;
          if (externalIp) {
            solution = chalk`Please ensure your configured IP address ${externalIp} is your public IP.
You can change it using {bold.cyanBright dashmate config set externalIp [IP]}.
Also, ensure that port ${port} on your public IP address is open
for incoming connections. You may need to configure your firewall to
ensure this port is accessible from the public internet. If you are using
Network Address Translation (NAT), please enable port forwarding for port 80
and all Dash service ports listed above.`;
          }

          const problem = new Problem(
            'Tenderdash P2P port is unavailable for incoming connections.',
            solution,
            SEVERITY.HIGH,
          );

          problems.push(problem);
        }
      }
    }

    return problems;
  }

  return analyseConfig;
}

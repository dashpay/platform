import chalk from 'chalk';
import { NETWORK_LOCAL, NETWORK_MAINNET } from '../../constants.js';
import { ERRORS } from '../../ssl/zerossl/validateZeroSslCertificateFactory.js';
import { SEVERITY } from '../Prescription.js';
import Problem from '../Problem.js';

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
                chalk`Please enable and set up SSL certificates {bold.cyanBright https://docs.dash.org/en/stable/masternodes/dashmate.html#ssl-certificate}`,
                SEVERITY.HIGH,
              );

              problems.push(problem);
            }
            break;
          case 'self-signed':
            if (config.get('network') === NETWORK_MAINNET) {
              const problem = new Problem(
                'Self-signed SSL certificate is used on mainnet. Clients won\'t be able to connect securely',
                chalk`Please use valid SSL certificates {bold.cyanBright https://docs.dash.org/en/stable/masternodes/dashmate.html#ssl-certificate}`,
                SEVERITY.HIGH,
              );

              problems.push(problem);
            }
            break;
          default: {
            const {
              description,
              solution,
            } = {
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

Or use ZeroSSL https://docs.dash.org/en/stable/masternodes/dashmate.html#ssl-certificate`,
              },
              // ZeroSSL validation errors
              [ERRORS.API_KEY_IS_NOT_SET]: {
                description: 'ZeroSSL API key is not set.',
                solution: chalk`Please obtain your API key from {underline.cyanBright https://app.zerossl.com/developer}
And then update your configuration with {block.cyanBright dashmate config set platform.gateway.ssl.providerConfigs.zerossl.apiKey [KEY]}`,
              },
              [ERRORS.EXTERNAL_IP_IS_NOT_SET]: {
                description: 'External IP is not set.',
                solution: chalk`Please update your configuration to include your external IP using {block.cyanBright dashmate config set externalIp [IP]}`,
              },
              [ERRORS.CERTIFICATE_ID_IS_NOT_SET]: {
                description: 'ZeroSSL certificate is not configured',
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain} to get a new certificate`,
              },
              [ERRORS.PRIVATE_KEY_IS_NOT_PRESENT]: {
                description: chalk`ZeroSSL private key file not found in ${ssl?.data?.privateKeyFilePath}.`,
                solution: chalk`Please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --force}
and revoke the previous certificate in the ZeroSSL dashboard`,
              },
              [ERRORS.EXTERNAL_IP_MISMATCH]: {
                description: chalk`ZeroSSL IP ${ssl?.data?.certificate.common_name} does not match external IP ${ssl?.data?.externalIp}.`,
                solution: chalk`Please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --force}
            and revoke the previous certificate in the ZeroSSL dashboard`,
              },
              [ERRORS.CSR_FILE_IS_NOT_PRESENT]: {
                description: chalk`ZeroSSL certificate request file not found in ${ssl?.data?.csrFilePath}.
This makes auto-renewal impossible.`,
                solution: chalk`If you need auto renew, please regenerate the certificate using {bold.cyanBright dashmate ssl obtain --force}
and revoke the previous certificate in the ZeroSSL dashboard`,
              },
              [ERRORS.CERTIFICATE_EXPIRES_SOON]: {
                description: chalk`ZeroSSL certificate expires at ${ssl?.data?.certificate.expires}.`,
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain} to get a new one`,
              },
              [ERRORS.CERTIFICATE_IS_NOT_VALIDATED]: {
                description: chalk`ZeroSSL certificate is not approved.`,
                solution: chalk`Please run {bold.cyanBright dashmate ssl obtain} to confirm certificate`,
              },
              [ERRORS.CERTIFICATE_IS_NOT_VALID]: {
                description: chalk`ZeroSSL certificate is not valid.`,
                solution: chalk`Please run {bold.cyanBright dashmate ssl zerossl obtain} to get a new one.`,
              },
              [ERRORS.ZERO_SSL_API_ERROR]: {
                description: ssl?.data?.error?.message,
                solution: chalk`Please contact ZeroSSL support if needed.`,
              },
            }[ssl.error] ?? {};

            if (description) {
              const problem = new Problem(
                description,
                solution,
                SEVERITY.HIGH,
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

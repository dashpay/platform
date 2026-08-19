import getBaseConfigFactory from '../../../../configs/defaults/getBaseConfigFactory.js';
import analyseConfigFactory from '../../../../src/doctor/analyse/analyseConfigFactory.js';
import { SEVERITY } from '../../../../src/doctor/Prescription.js';
import Samples from '../../../../src/doctor/Samples.js';
import { ERRORS as LETSENCRYPT_ERRORS } from '../../../../src/ssl/letsencrypt/validateLetsEncryptCertificateFactory.js';
import { ERRORS as ZEROSSL_ERRORS } from '../../../../src/ssl/zerossl/validateZeroSslCertificateFactory.js';

describe('analyseConfigFactory', () => {
  let analyseConfig;
  let config;
  let samples;

  /**
   * @param {Object} ssl
   * @param {string} [provider=zerossl]
   * @return {Problem[]}
   */
  function analyseSslSample(ssl, provider = 'zerossl') {
    config.set('platform.gateway.ssl.provider', provider);

    samples.setServiceInfo('gateway', 'ssl', ssl);

    return analyseConfig(samples);
  }

  beforeEach(() => {
    config = getBaseConfigFactory()();

    config.set('platform.enable', true);

    samples = new Samples();
    samples.setDashmateConfig(config);

    // Ports are reported healthy so that only certificate problems are analysed
    samples.setServiceInfo('core', 'p2pPort', 'OPEN');
    samples.setServiceInfo('gateway', 'httpPort', 'OPEN');
    samples.setServiceInfo('drive_tenderdash', 'p2pPort', 'OPEN');

    analyseConfig = analyseConfigFactory();
  });

  it('should report a problem for a Let\'s Encrypt certificate that expires soon', () => {
    const problems = analyseSslSample({
      error: LETSENCRYPT_ERRORS.CERTIFICATE_EXPIRES_SOON,
      data: { certificate: { expires: '2026-01-01' } },
    }, 'letsencrypt');

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('Let\'s Encrypt certificate expires at');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
  });

  it('should report a problem for a ZeroSSL certificate that expires soon', () => {
    const problems = analyseSslSample({
      error: ZEROSSL_ERRORS.CERTIFICATE_EXPIRES_SOON,
      data: { certificate: { expires: '2026-01-01' } },
    });

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('ZeroSSL certificate expires at');
    expect(problems[0].getSeverity()).to.equal(SEVERITY.HIGH);
  });

  it('should report a problem when the ZeroSSL API key is not set', () => {
    const problems = analyseSslSample({
      error: ZEROSSL_ERRORS.API_KEY_IS_NOT_SET,
      data: {},
    });

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('ZeroSSL API key is not set');
    expect(problems[0].getSolution()).to.include('dashmate config set platform.gateway.ssl.providerConfigs.zerossl.apiKey');
  });

  it('should report a problem when the external IP is not set', () => {
    const problems = analyseSslSample({
      error: ZEROSSL_ERRORS.EXTERNAL_IP_IS_NOT_SET,
      data: {},
    });

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('External IP is not set');
    expect(problems[0].getSolution()).to.include('dashmate config set externalIp');
  });

  it('should report a problem when certificate files are not found', () => {
    const problems = analyseSslSample({
      error: 'not-exist',
      data: {
        chainFilePath: '/home/dashmate/ssl/bundle.crt',
        privateFilePath: '/home/dashmate/ssl/private.key',
      },
    }, 'file');

    expect(problems).to.have.lengthOf(1);
    expect(problems[0].getDescription()).to.include('SSL certificate files are not found');
  });

  describe('ZeroSSL remediation', () => {
    it('should offer both renewing with ZeroSSL and switching to Let\'s Encrypt', () => {
      // Whether renewing works depends on the operator's ZeroSSL plan, which dashmate cannot
      // see, so both routes are offered rather than one being asserted to be the answer.
      const [problem] = analyseSslSample({
        error: ZEROSSL_ERRORS.CERTIFICATE_EXPIRES_SOON,
        data: { certificate: { expires: '2026-01-01' } },
      });

      expect(problem.getSolution()).to.include('dashmate ssl obtain');
      expect(problem.getSolution()).to.include('platform.gateway.ssl.provider letsencrypt');
    });

    it('should name what makes the alternative worth taking', () => {
      // Both providers renew on their own, so that is not what separates them
      const [problem] = analyseSslSample({
        error: ZEROSSL_ERRORS.CERTIFICATE_EXPIRES_SOON,
        data: { certificate: { expires: '2026-01-01' } },
      });

      expect(problem.getSolution()).to.include('free');
    });

    it('should surface the reason ZeroSSL itself gave', () => {
      const [problem] = analyseSslSample({
        error: ZEROSSL_ERRORS.ZERO_SSL_API_ERROR,
        data: { error: { message: 'Limit of certificates on your ZeroSSL account was reached' } },
      });

      expect(problem.getDescription()).to.include('Limit of certificates');
      expect(problem.getSolution()).to.include('platform.gateway.ssl.provider letsencrypt');
    });

    it('should still report an API failure that carried no message', () => {
      // The description doubles as the presence check, so an empty one dropped the problem
      const problems = analyseSslSample({
        error: ZEROSSL_ERRORS.ZERO_SSL_API_ERROR,
        data: {},
      });

      expect(problems).to.have.lengthOf(1);
    });

    it('should not suggest a command that does not exist', () => {
      const [problem] = analyseSslSample({
        error: ZEROSSL_ERRORS.CERTIFICATE_IS_NOT_VALID,
        data: {},
      });

      expect(problem.getSolution()).to.not.include('ssl zerossl obtain');
    });
  });

  it('should not report a problem for a valid certificate', () => {
    const problems = analyseSslSample({ data: {} });

    expect(problems).to.be.empty();
  });
});

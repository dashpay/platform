import qs from 'qs';
import Certificate from '../../../../src/ssl/zerossl/Certificate.js';
import cancelCertificate from '../../../../src/ssl/zerossl/cancelCertificate.js';
import createZeroSSLCertificate from '../../../../src/ssl/zerossl/createZeroSSLCertificate.js';
import downloadCertificate from '../../../../src/ssl/zerossl/downloadCertificate.js';
import getCertificate from '../../../../src/ssl/zerossl/getCertificate.js';
import listCertificates from '../../../../src/ssl/zerossl/listCertificates.js';
import requestApi from '../../../../src/ssl/zerossl/requestApi.js';
import revokeCertificate from '../../../../src/ssl/zerossl/revokeCertificate.js';
import verifyDomain from '../../../../src/ssl/zerossl/verifyDomain.js';

const apiKey = 'test-access-key';
const certificateId = 'certificate-id';
const certificateData = {
  id: certificateId,
  type: 1,
  status: 'issued',
  created: '2026-08-18 10:00:00',
  expires: '2026-11-16 10:00:00',
  common_name: '127.0.0.1',
};

function expectErrorNotToContain(error, values) {
  const serializedError = JSON.stringify({
    message: error.message,
    stack: error.stack,
    cause: error.cause,
    properties: Object.fromEntries(Object.entries(error)),
  });

  values.filter(Boolean).forEach((value) => {
    expect(serializedError).not.to.contain(value);
  });
}

describe('ZeroSSL API requests', () => {
  let fetchStub;

  beforeEach(function beforeEach() {
    fetchStub = this.sinon.stub(globalThis, 'fetch');
  });

  const requestCases = [
    {
      name: 'cancel a certificate',
      invoke: () => cancelCertificate(apiKey, certificateId),
      url: `https://api.zerossl.com/certificates/${certificateId}/cancel`,
      method: 'POST',
      contentType: 'application/x-www-form-urlencoded',
      response: { success: 1 },
      expectedResult: { success: 1 },
    },
    {
      name: 'create a certificate',
      invoke: () => createZeroSSLCertificate('certificate-csr', '127.0.0.1', apiKey),
      url: 'https://api.zerossl.com/certificates',
      method: 'POST',
      contentType: 'application/x-www-form-urlencoded',
      expectedBody: {
        certificate_domains: '127.0.0.1',
        certificate_validity_days: '90',
        certificate_csr: 'certificate-csr',
      },
      response: certificateData,
      assertResult: (result) => {
        expect(result).to.be.instanceOf(Certificate);
        expect(result.id).to.equal(certificateId);
        expect(result.created).to.be.instanceOf(Date);
        expect(result.expires).to.be.instanceOf(Date);
      },
    },
    {
      name: 'download a certificate',
      invoke: () => downloadCertificate(certificateId, apiKey),
      url: `https://api.zerossl.com/certificates/${certificateId}/download/return`,
      method: 'GET',
      response: {
        'certificate.crt': 'certificate',
        'ca_bundle.crt': 'ca-bundle',
      },
      expectedResult: 'certificate\nca-bundle',
    },
    {
      name: 'get a certificate',
      invoke: () => getCertificate(apiKey, certificateId),
      url: `https://api.zerossl.com/certificates/${certificateId}`,
      method: 'GET',
      response: certificateData,
      assertResult: (result) => {
        expect(result).to.be.instanceOf(Certificate);
        expect(result.id).to.equal(certificateId);
        expect(result.created).to.be.instanceOf(Date);
        expect(result.expires).to.be.instanceOf(Date);
      },
    },
    {
      name: 'list certificates with default filters',
      invoke: () => listCertificates(apiKey),
      url: 'https://api.zerossl.com/certificates?limit=1000&page=1',
      method: 'GET',
      response: { results: [certificateData] },
      assertResult: (result) => {
        expect(result).to.have.length(1);
        expect(result[0]).to.be.instanceOf(Certificate);
        expect(result[0].id).to.equal(certificateId);
        expect(result[0].created).to.be.instanceOf(Date);
        expect(result[0].expires).to.be.instanceOf(Date);
      },
    },
    {
      name: 'list certificates with populated filters',
      invoke: () => listCertificates(
        apiKey,
        ['draft', 'pending_validation'],
        2,
        'search-value',
      ),
      url: 'https://api.zerossl.com/certificates?limit=1000&page=2&statuses=draft,pending_validation&search=search-value',
      method: 'GET',
      response: { results: [] },
      expectedResult: [],
    },
    {
      name: 'revoke a certificate',
      invoke: () => revokeCertificate(apiKey, certificateId),
      url: `https://api.zerossl.com/certificates/${certificateId}/revoke`,
      method: 'POST',
      contentType: 'application/x-www-form-urlencoded',
      response: { success: 1 },
      expectedResult: { success: 1 },
    },
    {
      name: 'verify a domain',
      invoke: () => verifyDomain(certificateId, apiKey),
      url: `https://api.zerossl.com/certificates/${certificateId}/challenges`,
      method: 'POST',
      contentType: 'application/x-www-form-urlencoded',
      expectedBody: {
        validation_method: 'HTTP_CSR_HASH',
      },
      response: { success: 1 },
      expectedResult: { success: 1 },
    },
  ];

  requestCases.forEach((requestCase) => {
    it(`should use header authentication to ${requestCase.name}`, async function test() {
      fetchStub.resolves({
        json: this.sinon.stub().resolves(requestCase.response),
      });

      const result = await requestCase.invoke();

      expect(fetchStub).to.have.been.calledOnce();

      const [url, options] = fetchStub.firstCall.args;
      const headers = new Headers(options.headers);
      const authorizationHeaders = [...headers.entries()]
        .filter(([name]) => name.toLowerCase() === 'authorization');

      expect(url).to.equal(requestCase.url);
      expect(url).not.to.contain(apiKey);
      expect(url).not.to.contain('access_key');
      expect(options.method).to.equal(requestCase.method);
      expect(headers.has('Authorization')).to.equal(true);
      expect(headers.get('Authorization')).to.equal(`ApiKey ${apiKey}`);
      expect(authorizationHeaders).to.have.length(1);

      if (requestCase.contentType) {
        expect(headers.get('Content-Type')).to.equal(requestCase.contentType);
      }

      if (requestCase.expectedBody) {
        expect(qs.parse(options.body)).to.deep.equal(requestCase.expectedBody);
      } else {
        expect(options).not.to.have.property('body');
      }

      if (requestCase.assertResult) {
        requestCase.assertResult(result);
      } else {
        expect(result).to.deep.equal(requestCase.expectedResult);
      }
    });
  });

  describe('requestApi authentication boundary', () => {
    let consoleErrorStub;
    let consoleLogStub;
    let consoleWarnStub;

    beforeEach(function beforeEach() {
      consoleErrorStub = this.sinon.stub(console, 'error');
      consoleLogStub = this.sinon.stub(console, 'log');
      consoleWarnStub = this.sinon.stub(console, 'warn');
    });

    function expectNoConsoleCalls() {
      expect(consoleErrorStub).not.to.have.been.called();
      expect(consoleLogStub).not.to.have.been.called();
      expect(consoleWarnStub).not.to.have.been.called();
    }

    it('should preserve request options without mutating them', async function test() {
      const options = {
        method: 'POST',
        headers: {
          'Content-Type': 'application/x-www-form-urlencoded',
          'X-Request-ID': 'request-id',
        },
        body: 'payload',
        redirect: 'manual',
      };
      const originalOptions = structuredClone(options);
      fetchStub.resolves({ json: this.sinon.stub().resolves({ success: 1 }) });

      await requestApi(apiKey, 'https://api.zerossl.com/certificates', options);

      const [url, actualOptions] = fetchStub.firstCall.args;
      const headers = new Headers(actualOptions.headers);

      expect(url).to.equal('https://api.zerossl.com/certificates');
      expect(actualOptions).not.to.equal(options);
      expect(actualOptions.method).to.equal(options.method);
      expect(actualOptions.body).to.equal(options.body);
      expect(actualOptions.redirect).to.equal(options.redirect);
      expect(headers.get('Content-Type')).to.equal('application/x-www-form-urlencoded');
      expect(headers.get('X-Request-ID')).to.equal('request-id');
      expect(headers.get('Authorization')).to.equal(`ApiKey ${apiKey}`);
      expect(options).to.deep.equal(originalOptions);
      expectNoConsoleCalls();
    });

    ['authorization', 'AuThOrIzAtIoN'].forEach((headerName) => {
      it(`should replace a pre-existing ${headerName} header`, async function test() {
        fetchStub.resolves({ json: this.sinon.stub().resolves({ success: 1 }) });

        await requestApi(apiKey, 'https://api.zerossl.com/certificates', {
          method: 'GET',
          headers: { [headerName]: 'Bearer competing-credential' },
        });

        const headers = new Headers(fetchStub.firstCall.args[1].headers);
        const authorizationHeaders = [...headers.entries()]
          .filter(([name]) => name.toLowerCase() === 'authorization');

        expect(authorizationHeaders).to.deep.equal([
          ['authorization', `ApiKey ${apiKey}`],
        ]);
        expectNoConsoleCalls();
      });
    });

    [
      { name: 'missing', value: undefined },
      { name: 'empty', value: '' },
      { name: 'containing a newline', value: `${apiKey}\ninjected` },
      { name: 'with leading whitespace', value: ` ${apiKey}` },
      { name: 'with a leading tab', value: `\t${apiKey}` },
      { name: 'with trailing whitespace', value: `${apiKey} ` },
    ].forEach(({ name, value }) => {
      it(`should reject an API key ${name} before fetching`, async () => {
        let error;

        try {
          await requestApi(value, 'https://api.zerossl.com/certificates', {
            method: 'GET',
            headers: {},
          });
        } catch (e) {
          error = e;
        }

        expect(error).to.be.instanceOf(Error);
        expect(error.message).to.equal('Invalid ZeroSSL API key');
        expect(error).not.to.have.own.property('cause');
        expectErrorNotToContain(error, [value]);
        expect(fetchStub).not.to.have.been.called();
        expectNoConsoleCalls();
      });
    });

    it('should replace header-construction errors with a secret-free error', async () => {
      let error;

      try {
        await requestApi(apiKey, 'https://api.zerossl.com/certificates', {
          method: 'GET',
          headers: {
            'X-Invalid': `${apiKey}\ninvalid`,
          },
        });
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(Error);
      expect(error.message).to.equal('Invalid ZeroSSL API key');
      expect(error).not.to.have.own.property('cause');
      expectErrorNotToContain(error, [apiKey]);
      expect(fetchStub).not.to.have.been.called();
      expectNoConsoleCalls();
    });

    it('should redact the API key from parsed ZeroSSL errors before constructing an Error', async function test() {
      fetchStub.resolves({
        json: this.sinon.stub().resolves({
          error: {
            code: 999,
            type: `type ${apiKey}`,
            message: `message ${apiKey}`,
            details: {
              nested: [`details ${apiKey}`],
              [`property-${apiKey}`]: 'reflected property name',
            },
          },
        }),
      });

      let error;
      try {
        await requestApi(apiKey, 'https://api.zerossl.com/certificates', {
          method: 'GET',
          headers: {},
        });
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(Error);
      expect(error.code).to.equal(999);
      expect(error.message).to.equal('message [REDACTED]');
      expect(error.type).to.equal('type [REDACTED]');
      expect(error.details.nested).to.deep.equal(['details [REDACTED]']);
      expect(error.details['property-[REDACTED]']).to.equal('reflected property name');
      expectErrorNotToContain(error, [apiKey]);
      expectNoConsoleCalls();
    });

    it('should replace malformed JSON errors with a generic response error', async function test() {
      fetchStub.resolves({
        json: this.sinon.stub().rejects(
          new SyntaxError(`Unexpected token in ${apiKey} response`),
        ),
      });

      let error;
      try {
        await requestApi(apiKey, 'https://api.zerossl.com/certificates', {
          method: 'GET',
          headers: {},
        });
      } catch (e) {
        error = e;
      }

      expect(error).to.be.instanceOf(Error);
      expect(error.message).to.equal('Invalid ZeroSSL API response');
      expect(error).not.to.have.own.property('cause');
      expectErrorNotToContain(error, [apiKey, 'Unexpected token']);
      expectNoConsoleCalls();
    });
  });
});

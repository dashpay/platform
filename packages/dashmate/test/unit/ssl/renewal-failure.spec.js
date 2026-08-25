import { expect } from 'chai';
import classifyRenewalFailure, {
  describeRenewalFailure,
  RENEWAL_FAILURE_CODES,
  REMEDY_CLASS,
  sanitizeDetail,
} from '../../../src/ssl/renewal-failure.js';
import LegoArtifactsMissingError from '../../../src/ssl/errors/LegoArtifactsMissingError.js';
import LegoDidNotStartError from '../../../src/ssl/errors/LegoDidNotStartError.js';
import LegoResultNotObservedError from '../../../src/ssl/errors/LegoResultNotObservedError.js';

/**
 * What lego prints when Boulder could not reach the address at all.
 */
const LEGO_CONNECTION_FAILURE = `Failed to obtain Let's Encrypt certificate: Lego exited with code 1
2026/08/25 10:00:00 [INFO] [1.2.3.4] acme: Obtaining bundled SAN certificate
2026/08/25 10:00:00 [INFO] [1.2.3.4] AuthURL: https://acme-v02.api.letsencrypt.org/acme/authz-v3/98765
2026/08/25 10:00:05 [INFO] [1.2.3.4] acme: Trying to solve HTTP-01
2026/08/25 10:00:20 Could not obtain certificates:
\terror: one or more domains had a problem:
[1.2.3.4] acme: error: 400 :: urn:ietf:params:acme:error:connection :: 1.2.3.4: Fetching http://1.2.3.4/.well-known/acme-challenge/abc: Timeout during connect (likely firewall problem)`;

/**
 * What it prints when Boulder reached the address and got the wrong answer.
 */
const LEGO_WRONG_RESPONDER = `Failed to obtain Let's Encrypt certificate: Lego exited with code 1
[1.2.3.4] acme: error: 403 :: urn:ietf:params:acme:error:unauthorized :: 1.2.3.4: Invalid response from http://1.2.3.4/.well-known/acme-challenge/abc: 404`;

const LEGO_RATE_LIMITED = `Failed to obtain Let's Encrypt certificate: Lego exited with code 1
[1.2.3.4] acme: error: 429 :: urn:ietf:params:acme:error:rateLimited :: Error creating new order :: too many failed authorizations recently`;

describe('renewalFailure', () => {
  describe('classifyRenewalFailure', () => {
    describe("Let's Encrypt", () => {
      it('should name an unreachable port 80 from the problem the authority returned', () => {
        const { code } = classifyRenewalFailure(new Error(LEGO_CONNECTION_FAILURE));

        expect(code).to.equal(RENEWAL_FAILURE_CODES.PORT_80_UNREACHABLE);
      });

      it('should tell a wrong responder apart from an unreachable port', () => {
        // The distinction the guidance could not previously make. Both look
        // like "port 80 is broken" from outside, but one is a closed port and
        // the other is a web server answering in this node's place, and an
        // operator sent to open an already-open port never finds the second.
        const { code } = classifyRenewalFailure(new Error(LEGO_WRONG_RESPONDER));

        expect(code).to.equal(RENEWAL_FAILURE_CODES.PORT_80_WRONG_RESPONDER);
      });

      it('should name a rate limit, which must never invite another attempt', () => {
        const { code } = classifyRenewalFailure(new Error(LEGO_RATE_LIMITED));

        expect(code).to.equal(RENEWAL_FAILURE_CODES.RATE_LIMITED);
        expect(describeRenewalFailure(code).remedy).to.equal(REMEDY_CLASS.DO_NOT_RETRY);
      });

      it('should bucket an unrecognised problem as a refusal rather than guessing', () => {
        const { code, detail } = classifyRenewalFailure(new Error(
          '[1.2.3.4] acme: error: 500 :: urn:ietf:params:acme:error:serverInternal :: try later',
        ));

        expect(code).to.equal(RENEWAL_FAILURE_CODES.PROVIDER_REJECTED);
        // The type is kept so the reading is recoverable even though the
        // classifier had nothing to do with it.
        expect(detail).to.contain('serverInternal');
      });
    });

    describe('typed certificate helper failures', () => {
      // These arrive as the cause because the obtain task replaces them with
      // guidance written for a terminal. Without the cause none of them can be
      // established at all, and the whole Let's Encrypt half of the vocabulary
      // collapses into "could not work out why".
      it('should name a spent issuance that never landed', () => {
        const error = new Error('guidance text', {
          cause: new LegoArtifactsMissingError('/home/op/.dashmate/mainnet/x.crt'),
        });

        const { code } = classifyRenewalFailure(error);

        expect(code).to.equal(RENEWAL_FAILURE_CODES.CERTIFICATE_ISSUED_NOT_SAVED);
        expect(describeRenewalFailure(code).remedy).to.equal(REMEDY_CLASS.DO_NOT_RETRY);
      });

      it('should keep an unread result distinct from a spent issuance', () => {
        // Conflating them would either invite an attempt that spends a second
        // certificate, or forbid one when nothing was ever requested.
        const error = new Error('guidance text', {
          cause: new LegoResultNotObservedError(new Error('container vanished')),
        });

        expect(classifyRenewalFailure(error).code)
          .to.equal(RENEWAL_FAILURE_CODES.RESULT_UNKNOWN);
      });

      it('should name an occupied port 80, not an unreachable one', () => {
        // Opposite repairs: the port is reachable, it is taken.
        const error = new Error('guidance text', {
          cause: new LegoDidNotStartError(
            new Error('driver failed programming external connectivity: Bind for 0.0.0.0:80 failed: port is already allocated'),
          ),
        });

        expect(classifyRenewalFailure(error).code)
          .to.equal(RENEWAL_FAILURE_CODES.PORT_80_IN_USE);
      });

      it('should report any other failure to start as local, not as the authority refusing', () => {
        const error = new Error('guidance text', {
          cause: new LegoDidNotStartError(new Error('Cannot connect to the Docker daemon')),
        });

        expect(classifyRenewalFailure(error).code)
          .to.equal(RENEWAL_FAILURE_CODES.HELPER_DID_NOT_START);
      });
    });

    describe('ZeroSSL', () => {
      it('should name the free-tier certificate limit', () => {
        const error = Object.assign(
          new Error('Limit of certificates on your ZeroSSL account was reached'),
          { code: 2817, type: 'certificate_limit_reached' },
        );

        const { code } = classifyRenewalFailure(error);

        expect(code).to.equal(RENEWAL_FAILURE_CODES.QUOTA_EXHAUSTED);
        expect(describeRenewalFailure(code).remedy).to.equal(REMEDY_CLASS.SWITCH_PROVIDER);
      });

      it('should name a rejected account separately from a rejected request', () => {
        const error = Object.assign(new Error('ZeroSSL API key is invalid'), { code: 101 });

        expect(classifyRenewalFailure(error).code)
          .to.equal(RENEWAL_FAILURE_CODES.PROVIDER_AUTH);
      });

      it('should not claim which reading applies when its own check cannot tell', () => {
        // The server had already bound port 80 by then, so a local process
        // holding it is ruled out - but the check answers the same way when
        // nothing replied and when something replied with the wrong status, so
        // a proxy looks exactly like a closed port. Asserting either would
        // claim more than was observed.
        const error = new Error('Verification server is not responding.\nPlease ensure that port 80');

        expect(classifyRenewalFailure(error).code)
          .to.equal(RENEWAL_FAILURE_CODES.PORT_80_CHECK_FAILED);
      });
    });

    describe("dashmate's own failures", () => {
      it('should recognise a lost configuration lock instead of sending the operator to support', () => {
        const error = new Error('Lost the configuration lock while renewing the certificate,'
          + ' so the gateway service files were not written.');

        expect(classifyRenewalFailure(error).code)
          .to.equal(RENEWAL_FAILURE_CODES.RENEWAL_INTERRUPTED);
      });
    });

    describe('what it refuses to claim', () => {
      it('should return unknown and omit the excerpt when nothing recognisable was said', () => {
        // No line carried evidence, so there is nothing to quote. An arbitrary
        // slice of dashmate's own guidance would read as the provider's answer.
        const { code, detail } = classifyRenewalFailure(new Error('something went wrong'));

        expect(code).to.equal(RENEWAL_FAILURE_CODES.UNKNOWN);
        expect(detail).to.equal(null);
      });

      it('should not throw on something that is not an error at all', () => {
        expect(classifyRenewalFailure(undefined).code).to.equal(RENEWAL_FAILURE_CODES.UNKNOWN);
        expect(classifyRenewalFailure('a string').code).to.equal(RENEWAL_FAILURE_CODES.UNKNOWN);
        expect(classifyRenewalFailure({ message: 42 }).code).to.equal(RENEWAL_FAILURE_CODES.UNKNOWN);
      });
    });

    describe('what it writes down', () => {
      it('should take the excerpt from the message and from nothing else', () => {
        // ZeroSSL copies its whole response body onto the error, and a listr
        // failure can carry the task context - which on that path holds the
        // gateway's private key. Only the message may be read.
        const error = Object.assign(new Error('Your domain is not valid'), {
          code: 2808,
          details: { '1.2.3.4': { error_info: 'SECRET-DETAIL' } },
          ctx: { privateKeyFile: '-----BEGIN PRIVATE KEY-----MIIE' },
        });

        const { detail } = classifyRenewalFailure(error);

        expect(detail).to.equal('Your domain is not valid');
        expect(detail).to.not.contain('SECRET-DETAIL');
        expect(detail).to.not.contain('PRIVATE KEY');
      });

      it('should collapse the home directory before shortening, so a cut cannot leave a fragment of it', () => {
        // The reader's masking only matches the home directory where it ends,
        // so a value shortened partway through the operator's name would match
        // nothing and travel to whoever reads the report.
        const homeDirPath = '/home/alicebrown/.dashmate';
        const error = new Error(
          `[1.2.3.4] acme: error: 400 :: urn:ietf:params:acme:error:connection :: could not read ${homeDirPath}/mainnet/platform/gateway/ssl/bundle.crt while checking a very long path that keeps going`,
        );

        const { detail } = classifyRenewalFailure(error, { homeDirPath });

        expect(detail).to.not.contain('alicebro');
        expect(detail).to.contain('~');
      });

      it('should keep which authority answered but not which account asked', () => {
        const { detail } = classifyRenewalFailure(new Error(LEGO_CONNECTION_FAILURE));

        expect(detail).to.contain('urn:ietf:params:acme:error:connection');
        expect(detail).to.not.contain('/acme/authz-v3/98765');
      });

      it('should drop a contact address', () => {
        const { detail } = classifyRenewalFailure(new Error(
          'acme: error: 400 :: urn:ietf:params:acme:error:connection :: contact operator@example.com',
        ));

        expect(detail).to.not.contain('operator@example.com');
        expect(detail).to.contain('[email]');
      });

      it('should redact an API key the provider echoed back, whatever its own client missed', () => {
        // The provider's client redacts before throwing, but by exact substring
        // only - a key echoed back altered survives that pass.
        const { detail } = classifyRenewalFailure(
          Object.assign(new Error('rejected key SECRETKEY123 for this account'), { code: 2801 }),
          { apiKey: 'SECRETKEY123' },
        );

        expect(detail).to.not.contain('SECRETKEY123');
        expect(detail).to.contain('[REDACTED]');
      });

      it('should not carry back whatever page answered on port 80', () => {
        // The authority quotes what it fetched, and on the wrong-responder case
        // that is arbitrary content from a machine exposed to the internet.
        const { detail } = classifyRenewalFailure(new Error(
          '[1.2.3.4] acme: error: 403 :: urn:ietf:params:acme:error:unauthorized :: '
          + 'Invalid response from http://1.2.3.4/.well-known/x: "<html>session=SECRETCOOKIE</html>"',
        ));

        expect(detail).to.not.contain('SECRETCOOKIE');
        expect(detail).to.contain('unauthorized');
      });

      it('should stay within its length bound and on one line', () => {
        const { detail } = classifyRenewalFailure(new Error(
          `acme: error: 400 :: urn:ietf:params:acme:error:connection :: ${'x'.repeat(5000)}`,
        ));

        expect(detail.length).to.be.at.most(200);
        expect(detail).to.not.contain('\n');
      });

      it('should not spend unbounded time on one enormous line', () => {
        const started = Date.now();

        classifyRenewalFailure(new Error(`${'a'.repeat(10 * 1024 * 1024)} no urn here`));

        expect(Date.now() - started).to.be.below(1000);
      });
    });

    describe('sanitizeDetail', () => {
      it('should remove 8-bit control codes, which a terminal reads without an escape', () => {
        // U+009B is a control sequence introducer in its own right on a
        // terminal in 8-bit mode, so stripping only the 7-bit forms leaves the
        // channel open.
        const sanitized = sanitizeDetail(`before\u009B2Jafter\u0085`);

        expect(sanitized).to.not.contain('\u009B');
        expect(sanitized).to.not.contain('\u0085');
      });

      it('should remove terminal control sequences, which a report can carry from a stranger', () => {
        // `doctor --samples` renders an archive that arrived from someone else
        // into the terminal of whoever is helping. An escape left intact there
        // could rewrite what they see.
        const withEscape = `before\u001B[2Jafter\u0007`;

        const sanitized = sanitizeDetail(withEscape);

        expect(sanitized).to.not.contain('\u001B');
        expect(sanitized).to.not.contain('\u0007');
        expect(sanitized).to.equal('before [2Jafter');
      });
    });
  });

  describe('describeRenewalFailure', () => {
    it('should give every code a sentence and a remedy, so a new one cannot inherit the wrong ending', () => {
      Object.values(RENEWAL_FAILURE_CODES).forEach((code) => {
        const { sentence, remedy } = describeRenewalFailure(code);

        expect(sentence, code).to.be.a('string').and.have.length.above(0);
        expect(Object.values(REMEDY_CLASS), code).to.contain(remedy);
      });
    });

    it('should describe a code it does not know rather than printing the identifier', () => {
      // A report can be collected by a newer dashmate than the one reading it.
      // An identifier an operator cannot look up is worse than an admission.
      const { sentence, remedy } = describeRenewalFailure('SOMETHING_ADDED_LATER');

      expect(sentence).to.equal(describeRenewalFailure(RENEWAL_FAILURE_CODES.UNKNOWN).sentence);
      expect(remedy).to.equal(REMEDY_CLASS.SUPPORT);
    });
  });
});

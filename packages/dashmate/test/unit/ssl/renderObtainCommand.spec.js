import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import renderObtainCommand from '../../../src/ssl/renderObtainCommand.js';
import { SAFE_ACTION, ISSUANCE_STATUS } from '../../../src/ssl/renewalGuidance.js';

const SRC = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..', '..', 'src');

/**
 * The surfaces that report on a certificate and have the renewal guidance to
 * hand. Every one of them used to build the command itself and decide for
 * itself whether printing it was safe.
 */
const GUIDED_SURFACES = [
  'doctor/analyse/analyseGatewayCertificateFactory.js',
  'ssl/renderCertificateGuidance.js',
  'listr/tasks/update/gatewayCertificateTaskFactory.js',
];

describe('renderObtainCommand', () => {
  // The reason this module exists. Three rounds of review found branches that
  // printed a request the shared derivation had already withheld - a different
  // branch each time - and fixing them one at a time did not converge. A branch
  // that forgets to ask is now a failing test rather than a report an operator
  // acts on.
  it('should be the only way these surfaces can print a request', () => {
    // Comments are prose about the command, not the command. Only what the
    // file would actually print is the concern here.
    const withoutComments = (source) => source
      .split('\n')
      .filter((line) => !/^\s*(\/\/|\*|\/\*)/.test(line))
      .join('\n');

    const offenders = GUIDED_SURFACES.filter((file) => withoutComments(
      fs.readFileSync(path.join(SRC, file), 'utf8'),
    ).includes('ssl obtain'));

    expect(offenders, `build the command through renderObtainCommand instead: ${offenders}`)
      .to.deep.equal([]);
  });

  it('should give the command when nothing forbids it', () => {
    const rendered = renderObtainCommand({
      configName: 'mainnet',
      guidance: { safeAction: SAFE_ACTION.OBTAIN, issuanceStatus: ISSUANCE_STATUS.NONE },
    });

    expect(rendered).to.contain('dashmate ssl obtain --config mainnet');
  });

  // Not a bare refusal: the operator is told what to do instead, because a
  // problem that ends in nothing actionable is one they cannot act on.
  [
    ['a spent issuance', ISSUANCE_STATUS.SPENT, 'could not be saved'],
    ['an uncertain one', ISSUANCE_STATUS.UNCERTAIN, 'may already have been issued'],
    ['no reason it can name', ISSUANCE_STATUS.NONE, 'Send a report'],
  ].forEach(([name, issuanceStatus, expected]) => {
    it(`should withhold it and say why for ${name}`, () => {
      const rendered = renderObtainCommand({
        configName: 'mainnet',
        guidance: { safeAction: SAFE_ACTION.DO_NOT_OBTAIN, issuanceStatus },
      });

      expect(rendered).to.not.contain('ssl obtain');
      expect(rendered).to.contain(expected);
    });
  });

  it('should send a working node to the automatic attempt instead', () => {
    const rendered = renderObtainCommand({
      configName: 'mainnet',
      guidance: {
        safeAction: SAFE_ACTION.WAIT_AFTER_LOCAL_FIX,
        issuanceStatus: ISSUANCE_STATUS.NONE,
      },
    });

    expect(rendered).to.not.contain('ssl obtain');
    expect(rendered).to.contain('retries by itself');
  });

  // The failure this missed once: a caller that forgot to pass the config
  // rendered `--config undefined`, which an operator would paste verbatim and
  // run against the wrong node - or none.
  it('should never render a command without a config', () => {
    [undefined, null, ''].forEach((configName) => {
      expect(() => renderObtainCommand({
        configName,
        guidance: { safeAction: SAFE_ACTION.OBTAIN, issuanceStatus: ISSUANCE_STATUS.NONE },
      })).to.throw('needs the config');
    });
  });

  it('should carry the config into every command it prints', () => {
    const rendered = renderObtainCommand({
      configName: 'testnet',
      guidance: { safeAction: SAFE_ACTION.OBTAIN, issuanceStatus: ISSUANCE_STATUS.NONE },
      force: true,
    });

    expect(rendered).to.contain('--config testnet');
    expect(rendered).to.contain('--force');
  });
});

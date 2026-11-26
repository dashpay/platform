import parseTenderdashNodeKey from '../../../src/tenderdash/parseTenderdashNodeKey.js';
import generateTenderdashNodeKey from '../../../src/tenderdash/generateTenderdashNodeKey.js';
import deriveTenderdashNodeId from '../../../src/tenderdash/deriveTenderdashNodeId.js';

describe('parseTenderdashNodeKey', () => {
  let key;
  let id;

  /**
   * Every 8-character window of the key, so an error that quotes any part of
   * it is caught, not only one that quotes all of it
   *
   * @param {string} text
   * @return {string[]}
   */
  function keyFragmentsIn(text) {
    const fragments = [];
    for (let i = 0; i + 8 <= key.length; i += 1) {
      const fragment = key.slice(i, i + 8);
      if (text.includes(fragment)) {
        fragments.push(fragment);
      }
    }

    return fragments;
  }

  /**
   * @param {string} input
   * @return {Error}
   */
  function errorFor(input) {
    try {
      parseTenderdashNodeKey(input);
    } catch (e) {
      return e;
    }

    throw new Error('expected the input to be refused');
  }

  beforeEach(() => {
    key = generateTenderdashNodeKey();
    id = deriveTenderdashNodeId(key);
  });

  it('should accept the base64 key dashmate stores', () => {
    expect(parseTenderdashNodeKey(key)).to.deep.equal({ key, id });
  });

  // `dashmate config get platform.drive.tenderdash.node.key > file` ends the
  // file with a newline, and that is how the reset notice tells operators to
  // back the key up.
  it('should accept the key with surrounding whitespace, as a backup file has it', () => {
    expect(parseTenderdashNodeKey(`  ${key}\n`)).to.deep.equal({ key, id });
  });

  // The file Tenderdash and dashmate render, and what `tenderdash
  // gen-node-key` prints.
  it('should accept a node_key.json with a matching id', () => {
    const file = JSON.stringify({
      id,
      priv_key: { type: 'tendermint/PrivKeyEd25519', value: key },
    }, null, 2);

    expect(parseTenderdashNodeKey(file)).to.deep.equal({ key, id });
  });

  it('should accept a node_key.json without an id', () => {
    const file = JSON.stringify({
      priv_key: { type: 'tendermint/PrivKeyEd25519', value: key },
    });

    expect(parseTenderdashNodeKey(file)).to.deep.equal({ key, id });
  });

  it('should compare the id of a node_key.json case-insensitively', () => {
    const file = JSON.stringify({
      id: id.toUpperCase(),
      priv_key: { type: 'tendermint/PrivKeyEd25519', value: key },
    });

    expect(parseTenderdashNodeKey(file)).to.deep.equal({ key, id });
  });

  // An id that belongs to another key means the file was edited or assembled
  // by hand: which half is the one the operator meant cannot be known.
  it('should refuse a node_key.json whose id belongs to another key', () => {
    const file = JSON.stringify({
      id: deriveTenderdashNodeId(generateTenderdashNodeKey()),
      priv_key: { type: 'tendermint/PrivKeyEd25519', value: key },
    });

    expect(errorFor(file).message).to.match(/id .* does not match/i);
  });

  it('should refuse a node_key.json with another key type', () => {
    const file = JSON.stringify({
      id,
      priv_key: { type: 'tendermint/PrivKeySecp256k1', value: key },
    });

    expect(errorFor(file).message).to.match(/Ed25519/);
  });

  it('should refuse JSON that is not a node key file', () => {
    expect(errorFor('{"priv_key": 5}').message).to.match(/node_key\.json/);
  });

  // Node's JSON.parse quotes the input around the error position, so passing
  // its message on would print part of the key.
  it('should not quote the input when the JSON is malformed', () => {
    const error = errorFor(`{"priv_key": {"value": "${key}"`);

    expect(keyFragmentsIn(`${error.message}\n${error.stack}`)).to.be.empty();
  });

  it('should refuse an empty input', () => {
    expect(errorFor(' \n').message).to.match(/empty/);
  });

  it('should refuse a key of the wrong length', () => {
    const short = Buffer.from(key, 'base64').subarray(0, 32).toString('base64');

    expect(errorFor(short).message).to.match(/64 bytes/);
  });

  // Buffer.from(…, 'base64') skips characters it does not know, so the old
  // validator accepted a key with junk inside it and stored the junk.
  it('should refuse base64 with characters outside the alphabet', () => {
    const junk = `${key.slice(0, 10)}!${key.slice(10)}`;

    expect(errorFor(junk).message).to.match(/base64/);
  });

  it('should refuse base64 without its padding', () => {
    expect(errorFor(key.replace(/=+$/, '')).message).to.match(/base64/);
  });

  it('should refuse a key whose public half does not belong to its private half', () => {
    const other = Buffer.from(generateTenderdashNodeKey(), 'base64');
    const mixed = Buffer.concat([
      Buffer.from(key, 'base64').subarray(0, 32),
      other.subarray(32),
    ]).toString('base64');

    expect(errorFor(mixed).message).to.match(/public key/);
  });

  it('should never quote the input in any refusal', () => {
    const inputs = [
      `${key.slice(0, 10)}!${key.slice(10)}`,
      key.replace(/=+$/, ''),
      JSON.stringify({ id: 'ab'.repeat(20), priv_key: { type: 'tendermint/PrivKeyEd25519', value: key } }),
      JSON.stringify({ id, priv_key: { type: 'other', value: key } }),
      `{"priv_key": {"value": "${key}"`,
      `${key}${key}`,
    ];

    inputs.forEach((input) => {
      const error = errorFor(input);

      expect(keyFragmentsIn(`${error.message}\n${error.stack}`)).to.be.empty();
    });
  });
});

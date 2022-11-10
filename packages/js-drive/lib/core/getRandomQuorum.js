const BufferWriter = require('@dashevo/dashcore-lib/lib/encoding/bufferwriter');
const Hash = require('@dashevo/dashcore-lib/lib/crypto/hash');
const QuorumsNotFoundError = require('./errors/QuorumsNotFoundError');

const MIN_QUORUM_MEMBERS = 90;

/**
 * Calculates scores for validator quorum selection
 * it calculates sha256(hash, modifier) per quorumHash
 * Please note that this is not a double-sha256 but a single-sha256
 *
 * @param {Buffer[]} quorumHashes
 * @param {Buffer} modifier
 * @return {Object[]} scores
 */
function calculateQuorumHashScores(quorumHashes, modifier) {
  return quorumHashes.map((hash) => {
    const bufferWriter = new BufferWriter();

    bufferWriter.write(hash);
    bufferWriter.write(modifier);

    return { score: Hash.sha256(bufferWriter.toBuffer()), hash };
  });
}

/**
 * Filter quorum by the number of active members
 * @param {QuorumEntry[]} validatorQuorums
 * @return {QuorumEntry[]}
 */
function filterValidatorQuorumsByMembersAmount(validatorQuorums) {
  return validatorQuorums.filter(
    (validatorQuorum) => validatorQuorum.validMembersCount >= MIN_QUORUM_MEMBERS,
  );
}

/**
 * Gets the current validator set quorum hash for a particular core height
 *
 * @typedef {getRandomQuorum}
 * @param {SimplifiedMNList} sml
 * @param {number} quorumType
 * @param {Buffer} entropy - the entropy to select the quorum
 * @return {QuorumEntry} - the current validator set's quorumHash
 */
function getRandomQuorum(sml, quorumType, entropy) {
  const validatorQuorums = sml.getQuorumsOfType(quorumType);

  if (validatorQuorums.length === 0) {
    throw new QuorumsNotFoundError(sml, quorumType);
  }

  let filteredValidatorQuorums = filterValidatorQuorumsByMembersAmount(validatorQuorums);

  if (filteredValidatorQuorums.length === 0) {
    // use all validatorQuorums in this case
    filteredValidatorQuorums = validatorQuorums;
  }

  const validatorQuorumHashes = filteredValidatorQuorums
    .map((quorum) => Buffer.from(quorum.quorumHash, 'hex'));

  const scoredHashes = calculateQuorumHashScores(validatorQuorumHashes, entropy);

  scoredHashes.sort((a, b) => Buffer.compare(a.score, b.score));

  const quorumHash = scoredHashes[0].hash.toString('hex');

  return sml.getQuorum(quorumType, quorumHash);
}

module.exports = getRandomQuorum;

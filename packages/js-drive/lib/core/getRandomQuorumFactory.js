const BufferWriter = require('@dashevo/dashcore-lib/lib/encoding/bufferwriter');
const Hash = require('@dashevo/dashcore-lib/lib/crypto/hash');
const { LLMQ_TYPES } = require('@dashevo/dashcore-lib/lib/constants');
const QuorumsNotFoundError = require('./errors/QuorumsNotFoundError');
const ValidatorSet = require('../validator/ValidatorSet');

const MIN_QUORUM_VALID_MEMBERS = 90;

const LLMQ_TYPE_TO_NAME = Object
  .fromEntries(Object
    .entries(LLMQ_TYPES)
    .map(([key, value]) => [value, key.toLowerCase().replace('type_', '')]));

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
 *
 * @param {RpcClient} coreRpcClient
 * @return {getRandomQuorum}
 */
function getRandomQuorumFactory(coreRpcClient) {
  /**
   * Gets the current validator set quorum hash for a particular core height
   *
   * @typedef {getRandomQuorum}
   * @param {SimplifiedMNList} sml
   * @param {number} quorumType
   * @param {Buffer} entropy - the entropy to select the quorum
   * @param {number} coreHeight
   * @return return {Promise<QuorumEntry>} - the current validator set's quorumHash
   */
  async function getRandomQuorum(sml, quorumType, entropy, coreHeight) {
    const validatorQuorums = sml.getQuorumsOfType(quorumType);

    if (validatorQuorums.length === 0) {
      throw new QuorumsNotFoundError(sml, quorumType);
    }

    const { result: allValidatorQuorumsExtendedInfo } = await coreRpcClient.quorum('listextended', coreHeight);

    // convert to object
    const validatorQuorumsInfo = allValidatorQuorumsExtendedInfo[LLMQ_TYPE_TO_NAME[quorumType]]
      .reduce(
        (obj, item) => ({
          ...obj,
          ...item,
        }),
        {},
      );

    const numberOfQuorums = validatorQuorums.length;
    const minTtl = ValidatorSet.ROTATION_BLOCK_INTERVAL * 3;
    const dkgInterval = 24;

    // filter quorum by the number of valid members to choose the most vital ones
    let filteredValidatorQuorums = validatorQuorums
      .filter(
        (validatorQuorum) => validatorQuorum.validMembersCount >= MIN_QUORUM_VALID_MEMBERS,
      )
      .filter((validatorQuorum) => {
        const validatorQuorumInfo = validatorQuorumsInfo[validatorQuorum.quorumHash];

        if (!validatorQuorumInfo) {
          return false;
        }

        const quorumRemoveHeight = validatorQuorumInfo.creationHeight
          + (dkgInterval * numberOfQuorums);
        const howMuchInRest = quorumRemoveHeight - coreHeight;
        const quorumTtl = howMuchInRest * 2.5;

        return quorumTtl > minTtl;
      });

    if (filteredValidatorQuorums.length === 0) {
      // if there is no "vital" quorums, we choose among others with default min quorum size
      filteredValidatorQuorums = validatorQuorums;
    }

    const validatorQuorumHashes = filteredValidatorQuorums
      .map((quorum) => Buffer.from(quorum.quorumHash, 'hex'));

    const scoredHashes = calculateQuorumHashScores(validatorQuorumHashes, entropy);

    scoredHashes.sort((a, b) => Buffer.compare(a.score, b.score));

    const quorumHash = scoredHashes[0].hash.toString('hex');

    return sml.getQuorum(quorumType, quorumHash);
  }

  return getRandomQuorum;
}

module.exports = getRandomQuorumFactory;

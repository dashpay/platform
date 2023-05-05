const BufferWriter = require('@dashevo/dashcore-lib/lib/encoding/bufferwriter');
const Hash = require('@dashevo/dashcore-lib/lib/crypto/hash');
const { LLMQ_TYPES } = require('@dashevo/dashcore-lib/lib/constants');
const QuorumEntry = require('@dashevo/dashcore-lib/lib/deterministicmnlist/QuorumEntry');
const QuorumsNotFoundError = require('./errors/QuorumsNotFoundError');
const ValidatorSet = require('../validator/ValidatorSet');

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
 * @return {Object[]|null} scores
 */
function calculateQuorumHashScores(quorumHashes, modifier) {
  return quorumHashes.map((hash) => {
    const bufferWriter = new BufferWriter();

    bufferWriter.write(hash);
    bufferWriter.write(modifier);

    return { score: Hash.sha256(bufferWriter.toBuffer()), hash };
  });
}

const whiteList = [
  '34.214.48.68',
  '35.166.18.166',
  '35.165.50.126',
  '52.42.202.128',
  '52.12.176.90',
  '44.233.44.95',
  '35.167.145.149',
  '52.34.144.50',
  '44.240.98.102',
  '54.201.32.131',
  '52.10.229.11',
  '52.13.132.146',
  '44.228.242.181',
  '35.82.197.197',
  '52.40.219.41',
  '44.239.39.153',
  '54.149.33.167',
  '35.164.23.245',
  '52.33.28.47',
  '52.43.86.231',
  '52.43.13.92',
  '35.163.144.230',
  '52.89.154.48',
  '52.24.124.162',
  '44.227.137.77',
  '35.85.21.179',
  '54.187.14.232',
  '54.68.235.201',
  '52.13.250.182',
  '35.82.49.196',
  '44.232.196.6',
  '54.189.164.39',
  '54.213.204.85',
];

/**
 *
 * @param {RpcClient} coreRpcClient
 * @param {fetchQuorumMembers} fetchQuorumMembers
 * @param {SimplifiedMasternodeList} simplifiedMasternodeList
 * @return {getRandomQuorum}
 */
function getRandomQuorumFactory(coreRpcClient, fetchQuorumMembers, simplifiedMasternodeList) {
  /**
   * Gets the current validator set quorum hash for a particular core height
   *
   * @typedef {getRandomQuorum}
   * @param {SimplifiedMNList} sml
   * @param {number} quorumType
   * @param {Buffer} entropy - the entropy to select the quorum
   * @param {number} coreHeight
   * @returns {Promise<{ quorum: QuorumEntry, members: Object[]}|null>} - the current validator
   *  set's quorumHash
   */
  async function getRandomQuorum(sml, quorumType, entropy, coreHeight) {
    const validatorQuorums = sml.getQuorumsOfType(quorumType);

    if (validatorQuorums.length === 0) {
      throw new QuorumsNotFoundError(sml, quorumType);
    }

    const validMasternodesList = simplifiedMasternodeList
      .getStore()
      .getCurrentSML()
      .getValidMasternodesList();

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

    const quorumsMembers = {};

    await Promise.all(validatorQuorums.map(async (quorum) => {
      const members = await fetchQuorumMembers(
        quorumType,
        quorum.quorumHash,
      );

      quorumsMembers[quorum.quorumHash] = members.filter((member) => {
        // Remove invalid quorum members
        if (!member.valid) {
          return false;
        }

        // Ignore members which are banned
        const isValid = validMasternodesList
          .find((mnEntry) => mnEntry.proRegTxHash === member.proTxHash);

        if (!isValid) {
          return false;
        }

        // Remove non DCG nodes
        return whiteList.includes(member.service.split(':')[0]);
      });
    }));

    // filter quorum by the number of valid members to choose the most vital ones
    const filteredValidatorQuorums = validatorQuorums
      .filter(
        (validatorQuorum) => {
          const { threshold, size } = QuorumEntry.getParams(quorumType);

          const minimumNodeCount = Math.ceil((threshold / 100) * size);

          return quorumsMembers[validatorQuorum.quorumHash] >= minimumNodeCount;
        },
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
      return null;
    }

    const validatorQuorumHashes = filteredValidatorQuorums
      .map((quorum) => Buffer.from(quorum.quorumHash, 'hex'));

    const scoredHashes = calculateQuorumHashScores(validatorQuorumHashes, entropy);

    scoredHashes.sort((a, b) => Buffer.compare(a.score, b.score));

    const quorumHash = scoredHashes[0].hash.toString('hex');

    return {
      quorum: sml.getQuorum(quorumType, quorumHash),
      members: quorumsMembers[quorumHash],
    };
  }

  return getRandomQuorum;
}

module.exports = getRandomQuorumFactory;

module.exports = {
  isValidBlock(header) {
    return header.validProofOfWork() &&
        header.validTimestamp &&
        header.getDifficulty() > 0; // todo: do darkgravitywave
  },
};

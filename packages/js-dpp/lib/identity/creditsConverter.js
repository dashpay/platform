const RATIO = 1000;

function convertSatoshiToCredits(amount) {
  return amount * RATIO;
}

function convertCreditsToSatoshi(amount) {
  return Math.floor(amount / RATIO);
}

module.exports = {
  convertSatoshiToCredits,
  convertCreditsToSatoshi,
  RATIO,
};

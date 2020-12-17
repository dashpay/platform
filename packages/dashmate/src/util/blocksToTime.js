function blocksToTime(blocks) {
  let time;
  const blockTime = 2.625;
  const mins = blockTime * blocks;
  if (mins > 2880) {
    time = `${(mins / 60 / 24).toFixed(2)} days`;
  } else if (mins > 300) {
    time = `${(mins / 60).toFixed(2)} hours`;
  } else {
    time = `${(mins).toFixed(2)} minutes`;
  }
  return time;
}

module.exports = blocksToTime;

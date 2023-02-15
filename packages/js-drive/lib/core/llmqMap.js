module.exports = {
  0xff: 'llmq_none',

  1: 'llmq_50_60', // 50 members, 30 (60%) threshold, one per hour
  2: 'llmq_400_60', // 400 members, 240 (60%) threshold, one every 12 hours
  3: 'llmq_400_85', // 400 members, 340 (85%) threshold, one every 24 hours
  4: 'llmq_100_67', // 100 members, 67 (67%) threshold, one per hour
  5: 'llmq_60_75', // 60 members, 45 (75%) threshold, one every 12 hours

  // for testing only
  100: 'llmq_test', // 3 members, 2(66%) threshold, one per hour.

  // for devnets only
  101: 'llmq_devnet', // 12 members, 6 (50%) threshold, one per hour.

  // for testing activation of new quorums only
  102: 'llmq_test_v17', // 3 members, 2 (66%) threshold, one per hour.

  // for testing only
  103: 'llmq_test_dip0024', // 4 members, 2 (66%) threshold, one per hour.
  104: 'llmq_test_instantsend', // 3 members, 2 (66%) threshold, one per hour.

  // for devnets only. rotated version (v2) for devnets
  105: 'llmq_devnet_dip0024', // 8 members, 4 (50%) threshold, one per hour.
};

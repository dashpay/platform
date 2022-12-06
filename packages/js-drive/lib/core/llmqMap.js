module.exports = {
  llmq_none: 0xff,

  llmq_50_60: 1, // 50 members, 30 (60%) threshold, one per hour
  llmq_400_60: 2, // 400 members, 240 (60%) threshold, one every 12 hours
  llmq_400_85: 3, // 400 members, 340 (85%) threshold, one every 24 hours
  llmq_100_67: 4, // 100 members, 67 (67%) threshold, one per hour
  llmq_60_75: 5, // 60 members, 45 (75%) threshold, one every 12 hours

  // for testing only
  llmq_test: 100, // 3 members, 2(66%) threshold, one per hour.

  // for devnets only
  llmq_devnet: 101, // 12 members, 6 (50%) threshold, one per hour.

  // for testing activation of new quorums only
  llmq_test_v17: 102, // 3 members, 2 (66%) threshold, one per hour.

  // for testing only
  llmq_test_dip0024: 103, // 4 members, 2 (66%) threshold, one per hour.
  llmq_test_instantsend: 104, // 3 members, 2 (66%) threshold, one per hour.

  // for devnets only. rotated version (v2) for devnets
  llmq_devnet_dip0024: 105, // 8 members, 4 (50%) threshold, one per hour.
};

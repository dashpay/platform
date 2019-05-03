const config = {
  Api: {
    port: 3000,
  },
  grpc: {
    nativePort: 3010,
  },
  nullHash: '0000000000000000000000000000000000000000000000000000000000000000',
  MNListUpdateInterval: 60000,
  quorumUpdateInterval: 60000,
  DAPIDNSSeeds: [
    {
      proRegTxHash: 'fef106ff6420f9c6638c9676988a8fc655750caafb506c98cb5ff3d4fea99a41',
      confirmedHash: '0000000005d5635228f113b50fb5ad66995a7476ed20374e6e159f1f9e62347b',
      service: '127.0.0.1:19999',
      pubKeyOperator: '842476e8d82327adfb9b617a7ac3f62868946c0c4b6b0e365747cfb8825b8b79ba0eb1fa62e8583ae7102f59bf70c7c7',
      keyIDVoting: 'ca58159731cf7e3791958050d16bce02a64223ce',
      isValid: true,
    },
  ],
};

module.exports = config;

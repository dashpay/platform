// eslint-disable-next-line no-undef
rs.initiate({
  _id: 'driveDocumentIndices',
  version: 1,
  members: [
    {
      _id: 0,
      host: 'drive_mongodb:27017',
    },
  ],
});

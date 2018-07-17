/**
 * Attach StateView handlers
 *
 * @param {STHeadersReader} stHeadersReader
 * @param {applyStateTransition} applyStateTransition
 * @param {dropMongoDatabasesWithPrefix} dropMongoDatabasesWithPrefix
 */
function attachStateViewHandlers(
  stHeadersReader,
  applyStateTransition,
  dropMongoDatabasesWithPrefix,
) {
  stHeadersReader.on('header', async ({ header, block }) => {
    await applyStateTransition(header, block);
  });

  stHeadersReader.on('reset', async () => {
    await dropMongoDatabasesWithPrefix(process.env.MONGODB_DB_PREFIX);
  });
}

module.exports = attachStateViewHandlers;

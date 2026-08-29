const Dash = require('dash');
const { expect } = require('chai');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');
const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifier');
const waitForSTPropagated = require('../../../lib/waitForSTPropagated');

const {
  Errors: {
    StateTransitionBroadcastError,
  },
  PlatformProtocol: {
    Identifier,
  },
} = Dash;

describe('Platform', () => {
  describe('IndexOnlyDocument', function main() {
    this.timeout(900000);

    const POST_HASHTAG = 'dash';

    let client;
    let identity;
    let secondIdentity;
    let dataContract;
    let dataContractDocumentSchemas;
    let post;
    let like;
    let fetchedLike;

    before(async () => {
      dataContractDocumentSchemas = {
        post: {
          type: 'object',
          documentsMutable: false,
          canBeDeleted: false,
          indices: [
            {
              name: 'byHashtag',
              properties: [{ hashtag: 'asc' }],
            },
          ],
          properties: {
            hashtag: {
              type: 'string',
              minLength: 1,
              maxLength: 63,
              position: 0,
            },
            message: {
              type: 'string',
              maxLength: 280,
              position: 1,
            },
          },
          required: ['hashtag'],
          additionalProperties: false,
        },
        like: {
          type: 'object',
          indexOnly: true,
          documentsMutable: false,
          canBeDeleted: true,
          indices: [
            {
              name: 'byHashtagPost',
              properties: [{ hashtag: 'asc' }, { postId: 'asc' }],
              terminal: '$ownerId',
              countable: 'countable',
              rangeCountable: true,
              rankedCountable: true,
            },
            {
              name: 'byPost',
              properties: [{ postId: 'asc' }],
            },
            {
              name: 'byLiker',
              properties: [{ $ownerId: 'asc' }],
              terminal: 'postId',
            },
          ],
          properties: {
            hashtag: {
              type: 'string',
              minLength: 1,
              maxLength: 63,
              position: 0,
            },
            postId: {
              type: 'array',
              byteArray: true,
              minItems: 32,
              maxItems: 32,
              contentMediaType: Identifier.MEDIA_TYPE,
              refersTo: {
                type: 'permanentDocument',
                documentType: 'post',
                propertyAgreement: {
                  hashtag: 'hashtag',
                },
              },
              position: 1,
            },
          },
          required: ['hashtag', 'postId'],
          additionalProperties: false,
        },
      };

      client = await createClientWithFundedWallet(400000000); // 4 Dash

      identity = await client.platform.identities.register(200000000);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      secondIdentity = await client.platform.identities.register(100000000);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();
    });

    after(async () => {
      if (client) {
        await client.disconnect();
      }
    });

    it('should register a data contract with an indexOnly document type', async () => {
      dataContract = await client.platform.contracts.create(
        dataContractDocumentSchemas,
        identity,
      );

      await client.platform.contracts.publish(dataContract, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      client.getApps().set('yappr', {
        contractId: dataContract.getId(),
        contract: dataContract,
      });

      const fetchedDataContract = await client.platform.contracts.get(
        dataContract.getId(),
      );

      expect(fetchedDataContract.toObject()).to.be.deep.equal(dataContract.toObject());
      expect(fetchedDataContract.toObject().documentSchemas.like.indexOnly).to.be.true();
    });

    it('should fail to create a like that refers to a nonexistent post', async () => {
      const orphanedLike = await client.platform.documents.create(
        'yappr.like',
        identity,
        {
          hashtag: POST_HASHTAG,
          postId: await generateRandomIdentifier(),
        },
      );

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          create: [orphanedLike],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      // ReferencedEntityNotFoundError
      expect(broadcastError.code).to.equal(40120);
    });

    it('should fail to create a like whose hashtag disagrees with its post', async () => {
      post = await client.platform.documents.create(
        'yappr.post',
        identity,
        {
          hashtag: POST_HASHTAG,
          message: 'a post worth liking',
        },
      );

      await client.platform.documents.broadcast({
        create: [post],
      }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const disagreeingLike = await client.platform.documents.create(
        'yappr.like',
        identity,
        {
          hashtag: 'otherhashtag',
          postId: post.getId(),
        },
      );

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          create: [disagreeingLike],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      // ReferencedDocumentPropertyMismatchError: refersTo propertyAgreement
      // binds the like's hashtag to the referenced post's
      expect(broadcastError.code).to.equal(40127);
    });

    it('should create a like that agrees with its post', async () => {
      like = await client.platform.documents.create(
        'yappr.like',
        identity,
        {
          hashtag: POST_HASHTAG,
          postId: post.getId(),
        },
      );

      await client.platform.documents.broadcast({
        create: [like],
      }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();
    });

    it('should fail to create an identical like by the same identity', async () => {
      const duplicateLike = await client.platform.documents.create(
        'yappr.like',
        identity,
        {
          hashtag: POST_HASHTAG,
          postId: post.getId(),
        },
      );

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          create: [duplicateLike],
        }, identity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      // DuplicateUniqueIndexError: on an indexOnly type ANY existing entry
      // collides, so every index is a uniqueness constraint over its value
      // projection plus owner
      expect(broadcastError.code).to.equal(40105);
    });

    it('should synthesize likes from index entries when queried through a covering index', async () => {
      // A query through the [hashtag, postId] index covers every property
      // of the like (prefix properties from the path, the $ownerId
      // terminal from the member key), so the synthesized document is
      // complete
      const likes = await client.platform.documents.get(
        'yappr.like',
        {
          where: [
            ['hashtag', '==', POST_HASHTAG],
            ['postId', '==', post.getId()],
          ],
        },
      );

      expect(likes).to.have.lengthOf(1);

      const [fullLike] = likes;

      expect(fullLike.getOwnerId().toString()).to.equal(identity.getId().toString());
      expect(fullLike.get('hashtag')).to.equal(POST_HASHTAG);
      expect(fullLike.get('postId').toString()).to.equal(post.getId().toString());

      // The synthesized $id is deterministic over the proved index
      // position, NOT the id the create transition carried — nothing on
      // chain is ever addressed by it
      expect(fullLike.getId().toString()).to.not.equal(like.getId().toString());

      fetchedLike = fullLike;
    });

    it('should fail to query a subset-index projection without proofs', async () => {
      // The subset index [postId] synthesizes a projection without the
      // required hashtag; a partial document cannot be expressed in the
      // serialized non-proof response, so it only travels the proved read
      // surface (where the client synthesizes it from the proof itself)
      let fetchError;

      try {
        await client.platform.documents.get(
          'yappr.like',
          { where: [['postId', '==', post.getId()]] },
        );
      } catch (e) {
        fetchError = e;
      }

      expect(fetchError).to.exist();
      expect(fetchError.message).to.match(/does not cover every required property/);
    });

    it('should fail to fetch an indexOnly document by id', async () => {
      let fetchError;

      try {
        await client.platform.documents.get(
          'yappr.like',
          { where: [['$id', '==', fetchedLike.getId()]] },
        );
      } catch (e) {
        fetchError = e;
      }

      expect(fetchError).to.exist();
      expect(fetchError.message).to.match(/indexOnly documents cannot be fetched by id/);
    });

    it('should delete a like by its values', async () => {
      // The delete carries the document's full value tuple (there is no
      // stored row and no id to delete by), so it must be built from a
      // document fetched through an index that covers every property
      await client.platform.documents.broadcast({
        delete: [fetchedLike],
      }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const likes = await client.platform.documents.get(
        'yappr.like',
        {
          where: [
            ['hashtag', '==', POST_HASHTAG],
            ['postId', '==', post.getId()],
          ],
        },
      );

      expect(likes).to.have.lengthOf(0);
    });

    it('should allow a second identity to like the same post but not to delete another identity\'s like', async () => {
      // Re-create the first identity's like so the second identity's
      // identical values face an existing entry
      like = await client.platform.documents.create(
        'yappr.like',
        identity,
        {
          hashtag: POST_HASHTAG,
          postId: post.getId(),
        },
      );

      await client.platform.documents.broadcast({
        create: [like],
      }, identity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      // Deletes are self-authorizing: the network computes the entry tuple
      // with owner = signer, so the second identity can only ever address
      // its own entries and the first identity's like surfaces as not found
      const [firstIdentityLike] = await client.platform.documents.get(
        'yappr.like',
        {
          where: [
            ['hashtag', '==', POST_HASHTAG],
            ['postId', '==', post.getId()],
          ],
        },
      );

      firstIdentityLike.setOwnerId(secondIdentity.getId());

      let broadcastError;

      try {
        await client.platform.documents.broadcast({
          delete: [firstIdentityLike],
        }, secondIdentity);
      } catch (e) {
        broadcastError = e;
      }

      expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
      // DocumentNotFoundError
      expect(broadcastError.code).to.equal(40101);

      // Identical values under a different terminal are a different entry,
      // not a duplicate
      const secondLike = await client.platform.documents.create(
        'yappr.like',
        secondIdentity,
        {
          hashtag: POST_HASHTAG,
          postId: post.getId(),
        },
      );

      await client.platform.documents.broadcast({
        create: [secondLike],
      }, secondIdentity);

      // Additional wait time to mitigate testnet latency
      await waitForSTPropagated();

      const likes = await client.platform.documents.get(
        'yappr.like',
        {
          where: [
            ['hashtag', '==', POST_HASHTAG],
            ['postId', '==', post.getId()],
          ],
        },
      );

      expect(likes).to.have.lengthOf(2);
      expect(likes.map((doc) => doc.getOwnerId().toString())).to.have.members([
        identity.getId().toString(),
        secondIdentity.getId().toString(),
      ]);
    });
  });
});

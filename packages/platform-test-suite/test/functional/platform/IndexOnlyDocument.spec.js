const Dash = require('dash');
const { expect } = require('chai');

const createClientWithFundedWallet = require('../../../lib/test/createClientWithFundedWallet');
const generateRandomIdentifier = require('../../../lib/test/utils/generateRandomIdentifier');
const waitForSTPropagated = require('../../../lib/waitForSTPropagated');
const createPlatformProofVerifier = require('../../../lib/test/createPlatformProofVerifier');

function identifierLikeToBase58(evo, value) {
  if (typeof value === 'string') {
    return value;
  }

  if (value && typeof value.toBase58 === 'function') {
    return value.toBase58();
  }

  return evo.Identifier.fromBytes(Array.from(value)).toBase58();
}

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
              // hashtag is this index's skip trigger: a like that omits
              // it writes no byHashtagPost entry at all
              skipIfAbsent: true,
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
          required: ['postId'],
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

    it('should fetch liked posts through a chained query with verified proofs', async () => {
      // The provable semi-join: SELECT * FROM post WHERE $id IN
      // (SELECT postId FROM like WHERE $ownerId = me). Served by the
      // dedicated getChainedDocuments endpoint through the WASM SDK,
      // which verifies ONE merged grovedb proof against the
      // quorum-signed root, re-deriving the outer query and checking
      // it against the proven inner values — the node cannot steer
      // the join.
      const { evo, sdk: evoSdk } = await createPlatformProofVerifier
        .getEvoSdkForNetwork(process.env.NETWORK);

      const page = await evoSdk.documents.chained({
        dataContractId: dataContract.getId().toString(),
        innerDocumentType: 'like',
        where: [['$ownerId', '==', identity.getId().toString()]],
        innerLimit: 10,
        joinProperty: 'postId',
        outerDocumentType: 'post',
      });

      expect(page.innerDocuments).to.have.lengthOf(1);
      expect(page.outerDocuments).to.have.lengthOf(1);

      const [likedPost] = page.outerDocuments;
      expect(likedPost.id.toBase58()).to.equal(post.getId().toString());
      expect(likedPost.properties.message).to.equal('a post worth liking');

      // The inner projection carries the pagination cursor.
      const [innerLike] = page.innerDocuments;
      expect(identifierLikeToBase58(evo, innerLike.properties.postId))
        .to.equal(post.getId().toString());
    });

    it('should fetch a feed page with its like counts and my likes through a composite query', async () => {
      // A page plus the sub-queries derived from it, ONE merged proof:
      // the dash posts, one like count per post (from the countable
      // [hashtag, postId] index with hashtag fixed), and which of them
      // I liked (the byLiker index with $ownerId fixed, its postId
      // terminal bound to the page ids: value-bounded, so no limit).
      // The WASM SDK bootstraps the page from the proof, re-derives
      // every sub-query, and verifies the composition against the
      // quorum-signed root.
      const { sdk: evoSdk } = await createPlatformProofVerifier
        .getEvoSdkForNetwork(process.env.NETWORK);

      const page = await evoSdk.documents.composite({
        dataContractId: dataContract.getId().toString(),
        documentType: 'post',
        where: [['hashtag', '==', POST_HASHTAG]],
        limit: 10,
        subQueries: [
          {
            documentType: 'like',
            kind: 'counts',
            where: [['hashtag', '==', POST_HASHTAG]],
            bind: { sourceProperty: '$id', field: 'postId' },
          },
          {
            documentType: 'like',
            where: [['$ownerId', '==', identity.getId().toString()]],
            bind: { sourceProperty: '$id', field: 'postId' },
          },
        ],
      });

      expect(page.pageDocuments).to.have.lengthOf(1);
      expect(page.subResults).to.have.lengthOf(2);

      const [pagePost] = page.pageDocuments;
      expect(pagePost.id.toBase58()).to.equal(post.getId().toString());

      const [likeCounts, myLikes] = page.subResults;
      expect(likeCounts.kind).to.equal('counts');
      expect(likeCounts.counts.get(post.getId().toString())).to.equal(1n);

      expect(myLikes.kind).to.equal('documents');
      expect(myLikes.documents).to.have.lengthOf(1);
      expect(myLikes.documents[0].ownerId.toBase58()).to.equal(identity.getId().toString());
    });

    it('should fail to query a subset-index projection without proofs', async () => {
      // The subset index [postId] synthesizes a projection without the
      // hashtag — and with hashtag optional, serializing it would assert
      // an absence the index cannot know; partial documents only travel
      // the proved read surface (where the client synthesizes them from
      // the proof itself)
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
      expect(fetchError.message).to.match(/does not cover every property/);
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

    describe('skipIfAbsent', () => {
      let untaggedPost;
      let untaggedLike;
      let freshTaggedPost;

      it('should create an untagged post and an untagged like under the absence agreement', async () => {
        // post.hashtag is optional; a post may carry no tag at all
        untaggedPost = await client.platform.documents.create(
          'yappr.post',
          identity,
          {
            message: 'a post with no hashtag',
          },
        );

        await client.platform.documents.broadcast({
          create: [untaggedPost],
        }, identity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        // Both sides of the propertyAgreement absent: the like may omit
        // its hashtag exactly because the post has none — and the
        // skipIfAbsent byHashtagPost index writes nothing for it
        untaggedLike = await client.platform.documents.create(
          'yappr.like',
          identity,
          {
            postId: untaggedPost.getId(),
          },
        );

        await client.platform.documents.broadcast({
          create: [untaggedLike],
        }, identity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();
      });

      it('should refuse a hashtag-less like on a tagged post', async () => {
        // A FRESH tagged post: both identities already hold likes on the
        // shared `post`, and the create-side duplicate probe would refuse
        // the colliding byPost entry (40105) before the agreement check
        // ever ran — the shape under test needs a post this identity has
        // no like on
        freshTaggedPost = await client.platform.documents.create(
          'yappr.post',
          identity,
          {
            hashtag: POST_HASHTAG,
            message: 'a second tagged post',
          },
        );

        await client.platform.documents.broadcast({
          create: [freshTaggedPost],
        }, identity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        // Referring absent, referenced present: the absence agreement is
        // strict — a like on a tagged post must carry the tag, or per-tag
        // aggregates would silently deflate
        const hashtagLessLike = await client.platform.documents.create(
          'yappr.like',
          secondIdentity,
          {
            postId: freshTaggedPost.getId(),
          },
        );

        let broadcastError;

        try {
          await client.platform.documents.broadcast({
            create: [hashtagLessLike],
          }, secondIdentity);
        } catch (e) {
          broadcastError = e;
        }

        expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
        // ReferencedDocumentPropertyMismatchError
        expect(broadcastError.code).to.equal(40127);
      });

      it('should refuse a tagged like on an untagged post', async () => {
        // Referring present, referenced absent: the mirror mismatch
        const taggedLike = await client.platform.documents.create(
          'yappr.like',
          secondIdentity,
          {
            hashtag: POST_HASHTAG,
            postId: untaggedPost.getId(),
          },
        );

        let broadcastError;

        try {
          await client.platform.documents.broadcast({
            create: [taggedLike],
          }, secondIdentity);
        } catch (e) {
          broadcastError = e;
        }

        expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
        // ReferencedDocumentPropertyMismatchError
        expect(broadcastError.code).to.equal(40127);
      });

      it('should keep untagged likes out of the hashtag index', async () => {
        // The skip index is a sparse projection: it holds exactly the
        // likes that carry a hashtag, so the untagged like is invisible
        // to per-hashtag queries (which must bind the trigger)
        const likes = await client.platform.documents.get(
          'yappr.like',
          {
            where: [
              ['hashtag', '==', POST_HASHTAG],
              ['postId', '==', untaggedPost.getId()],
            ],
          },
        );

        expect(likes).to.have.lengthOf(0);
      });

      it('should delete an untagged like by its values', async () => {
        // The locally created document carries the exact value tuple
        // (postId only) — the delete recomputes the same skip, removing
        // entries from the non-skip indexes alone
        await client.platform.documents.broadcast({
          delete: [untaggedLike],
        }, identity);

        // Additional wait time to mitigate testnet latency
        await waitForSTPropagated();

        let broadcastError;

        try {
          await client.platform.documents.broadcast({
            delete: [untaggedLike],
          }, identity);
        } catch (e) {
          broadcastError = e;
        }

        expect(broadcastError).to.be.an.instanceOf(StateTransitionBroadcastError);
        // DocumentNotFoundError: the first delete was exact
        expect(broadcastError.code).to.equal(40101);
      });
    });
  });
});

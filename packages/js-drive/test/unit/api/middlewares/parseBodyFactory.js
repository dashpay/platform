const { PassThrough } = require('stream');
const parseBodyFactory = require('../../../../lib/api/middlewares/parseBodyFactory');

describe('parseBodyFactory', () => {
  const parseBody = parseBodyFactory();

  it('should parse body from request stream', (done) => {
    const request = new PassThrough();
    const params = { method: 'fetchDapObjects', params: ['123456'] };

    parseBody(request, null, () => {
      expect(request.body).to.be.deep.equal(params);
      done();
    });

    request.end(JSON.stringify(params));
  });

  it('should call next in case of error');
});

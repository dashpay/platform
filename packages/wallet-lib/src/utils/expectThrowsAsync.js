// eslint-disable-next-line import-x/no-extraneous-dependencies
import { expect } from 'chai';

const expectThrowsAsync = async (method, errorMessage) => {
  let error = null;
  try {
    const res = await method();
    expect(res).to.be.an('Error');
    if (errorMessage) {
      if (res.message) {
        error = res;
      }
    }
  } catch (err) {
    error = err;
  }
  expect(error).to.be.an('Error');
  if (errorMessage) {
    expect(error.message).to.equal(errorMessage);
  }
};
export default expectThrowsAsync;
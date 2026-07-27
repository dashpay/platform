import chai from 'chai';
import chaiAsPromised from 'chai-as-promised';
import dirtyChai from 'dirty-chai';

chai.use(chaiAsPromised);
chai.use(dirtyChai);

export const { expect } = chai;
export default chai;

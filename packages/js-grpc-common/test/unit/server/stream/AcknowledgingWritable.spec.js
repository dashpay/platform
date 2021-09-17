const WritableMock = require('../../../../lib/test/mock/WritableMock');
const AcknowledgingWritable = require('../../../../lib/server/stream/AcknowledgingWritable');

describe('AcknowledgingWritable', () => {
  describe('#write', () => {
    it('should throw when wrapped stream emits error', () => {
      const writable = new WritableMock({ fireOnErrorWithoutCallback: true });
      const wrapper = new AcknowledgingWritable(writable);

      expect(wrapper.write('123')).to.be.rejectedWith('Error event');
    });

    it('should throw when wrapped stream calls callback with an error', () => {
      const writable = new WritableMock({ callWriteCallbackWithAnError: true });
      const wrapper = new AcknowledgingWritable(writable);

      expect(wrapper.write('123')).to.be.rejectedWith('Error from callback');
    });

    it('should throw an error if .write method of the wrapped stream throws an error', () => {
      const writable = new WritableMock({ throwInWrite: true });
      const wrapper = new AcknowledgingWritable(writable);

      expect(wrapper.write('123')).to.be.rejectedWith('Thrown error');
    });

    it('should return true when wrapped ._write callback called', async () => {
      const writable = new WritableMock({ callCallback: true });
      const wrapper = new AcknowledgingWritable(writable);

      const result = await wrapper.write('123');

      expect(result).to.be.true();
    });

    it("should attach handlers when write is called and detach when it's finished", async () => {
      const writable = new WritableMock({ callCallback: true });
      const wrapper = new AcknowledgingWritable(writable);

      // eslint-disable-next-line no-underscore-dangle
      expect(wrapper.writable._eventsCount).to.be.equal(0);

      const promise = wrapper.write('123');

      // eslint-disable-next-line no-underscore-dangle
      expect(wrapper.writable._eventsCount).to.be.equal(2);

      const result = await promise;

      expect(result).to.be.true();

      // eslint-disable-next-line no-underscore-dangle
      expect(wrapper.writable._eventsCount).to.be.equal(0);
    });

    it('should return true when instead of calling callback drain is required', async () => {
      const writable = new WritableMock({ requireDrain: true });
      const wrapper = new AcknowledgingWritable(writable);

      // eslint-disable-next-line no-underscore-dangle
      expect(wrapper.writable._eventsCount).to.be.equal(0);

      const promise = wrapper.write('123');

      // event error is still not detached
      // eslint-disable-next-line no-underscore-dangle
      expect(wrapper.writable._eventsCount).to.be.equal(1);

      const result = await promise;

      expect(result).to.be.true();

      // eslint-disable-next-line no-underscore-dangle
      expect(wrapper.writable._eventsCount).to.be.equal(0);
    });
  });
});

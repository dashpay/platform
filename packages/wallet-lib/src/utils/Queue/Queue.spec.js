const { expect } = require('chai');
const Queue = require('./Queue');
const Job = require('./Job');

async function storeDataAtSomePoint(storage, data) {
  return new Promise((resolve) => {
    const timeoutValue = Math.ceil(Math.random() * 1000);
    setTimeout(() => {
      storage.push(data);
      resolve(data);
    }, timeoutValue);
  })
}

let store;
let queue;
describe('Utils - Queue', function suite() {
  this.timeout(5000);
  const processedResults = [];
  it('should instantiate a Queue', function () {
    queue = new Queue({});
    expect(queue).to.exist;
    store = [];
  })
  it('should enqueue and process job', function (done) {
    queue.on('processed', ({result}) => {
      processedResults.push(result);
      if (processedResults.length === 3) {
        expect(processedResults).to.deep.equal([{id: 1}, {id: 2}, {id: 3}]);
        done();
      }
    });
    const job1 = new Job(1, storeDataAtSomePoint.bind(null, store, {id: 1}));
    queue.enqueueJob(job1);
    const job2 = new Job(2, storeDataAtSomePoint.bind(null, store, {id: 2}));
    queue.enqueueJob(job2);
    const job3 = new Job(3, storeDataAtSomePoint.bind(null, store, {id: 3}));
    queue.enqueueJob(job3);
  });
  it('should process next enqueued', function (done) {
    const job5 = new Job(5, storeDataAtSomePoint.bind(null, store, {id: 5}))
    const job6 = new Job(6, storeDataAtSomePoint.bind(null, store, {id: 6}))
    const job4 = new Job(4, storeDataAtSomePoint.bind(null, store, {id: 4}));
    queue.enqueueJob(job4);
    queue.enqueueJob(job5);
    queue.enqueueJob(job6);
    queue.on('processed', () => {
      if (processedResults.length === 6) done();
    })
  });
  it('should have correctly dealt with order', function () {
    expect(processedResults.length).to.equal(6);
    expect(processedResults).to.deep.equal([{id: 1}, {id: 2}, {id: 3}, {id: 4}, {id: 5}, {id: 6}]);
  });
  it('should catch and emit the async error from jobs to queue', function (done) {
    const asyncFnThrowing = async () => {
      throw new Error('An error from job');
    }
    queue.on('error', (e)=>{
      expect(e.message).to.equal('An error from job');
      done()
    })
    const job7 = new Job(7, asyncFnThrowing);
    queue.enqueueJob(job7);

  });
});

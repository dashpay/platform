class Job {
  constructor(id, fn) {
    this.id = id;
    this.fn = fn;
    this.timestamp = Date.now();
  }
}
module.exports = Job;

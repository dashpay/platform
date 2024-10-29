export default class Problem {
  /**
   * @type {string}
   */
  #description;

  /**
   * @type {string}
   */
  #solution;

  /**
   * @type {number}
   */
  #severity;

  /**
   * @param {string} description
   * @param {string} solution
   * @param {number} severity
   */
  constructor(description, solution, severity) {
    this.#description = description;
    this.#solution = solution;
    this.#severity = severity;
  }

  /**
   * @return {string}
   */
  getDescription() {
    return this.#description;
  }

  /**
   * @return {string}
   */
  getSolution() {
    return this.#solution;
  }

  /**
   * @return {number}
   */
  getSeverity() {
    return this.#severity;
  }
}

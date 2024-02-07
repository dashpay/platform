import { fibonacci } from '../../../../utils/fibonacci';
import { wait } from '../../../../utils/wait';

/**
 * Maximum number of retry attempts
 */

const withRetry = async <T>(
  query: (...args: any[]) => Promise<T>,
  maxAttempts: number,
  delayMulMs: number,
): Promise<T> => {
  let attempt = 0;
  let result;

  if (maxAttempts < 1) {
    throw new Error('maxAttempts must be greater than 0');
  }

  while (attempt < maxAttempts) {
    try {
      result = await query();
      break;
    } catch (e) {
      attempt += 1;
      if (attempt >= maxAttempts) {
        throw e;
      }
      const delay = fibonacci(attempt) * delayMulMs;
      await wait(delay);
    }
  }

  return result;
};

export default withRetry;

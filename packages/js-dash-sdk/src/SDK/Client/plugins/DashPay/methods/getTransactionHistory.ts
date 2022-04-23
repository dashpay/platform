export async function getTransactionHistory(this: any){
  const account = await this.platform.client.getWalletAccount();
  const transactionHistory = account.getTransactionHistory();
  return transactionHistory;
}

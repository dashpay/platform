#import "Core.pbobjc.h"

#import <ProtoRPC/ProtoService.h>
#import <ProtoRPC/ProtoRPC.h>
#import <RxLibrary/GRXWriteable.h>
#import <RxLibrary/GRXWriter.h>



NS_ASSUME_NONNULL_BEGIN

@protocol Core <NSObject>

#pragma mark getStatus(GetStatusRequest) returns (GetStatusResponse)

- (void)getStatusWithRequest:(GetStatusRequest *)request handler:(void(^)(GetStatusResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCTogetStatusWithRequest:(GetStatusRequest *)request handler:(void(^)(GetStatusResponse *_Nullable response, NSError *_Nullable error))handler;


#pragma mark getBlock(GetBlockRequest) returns (GetBlockResponse)

- (void)getBlockWithRequest:(GetBlockRequest *)request handler:(void(^)(GetBlockResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCTogetBlockWithRequest:(GetBlockRequest *)request handler:(void(^)(GetBlockResponse *_Nullable response, NSError *_Nullable error))handler;


#pragma mark broadcastTransaction(BroadcastTransactionRequest) returns (BroadcastTransactionResponse)

- (void)broadcastTransactionWithRequest:(BroadcastTransactionRequest *)request handler:(void(^)(BroadcastTransactionResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCTobroadcastTransactionWithRequest:(BroadcastTransactionRequest *)request handler:(void(^)(BroadcastTransactionResponse *_Nullable response, NSError *_Nullable error))handler;


#pragma mark getTransaction(GetTransactionRequest) returns (GetTransactionResponse)

- (void)getTransactionWithRequest:(GetTransactionRequest *)request handler:(void(^)(GetTransactionResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCTogetTransactionWithRequest:(GetTransactionRequest *)request handler:(void(^)(GetTransactionResponse *_Nullable response, NSError *_Nullable error))handler;


#pragma mark getEstimatedTransactionFee(GetEstimatedTransactionFeeRequest) returns (GetEstimatedTransactionFeeResponse)

- (void)getEstimatedTransactionFeeWithRequest:(GetEstimatedTransactionFeeRequest *)request handler:(void(^)(GetEstimatedTransactionFeeResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCTogetEstimatedTransactionFeeWithRequest:(GetEstimatedTransactionFeeRequest *)request handler:(void(^)(GetEstimatedTransactionFeeResponse *_Nullable response, NSError *_Nullable error))handler;


#pragma mark subscribeToBlockHeadersWithChainLocks(BlockHeadersWithChainLocksRequest) returns (stream BlockHeadersWithChainLocksResponse)

- (void)subscribeToBlockHeadersWithChainLocksWithRequest:(BlockHeadersWithChainLocksRequest *)request eventHandler:(void(^)(BOOL done, BlockHeadersWithChainLocksResponse *_Nullable response, NSError *_Nullable error))eventHandler;

- (GRPCProtoCall *)RPCTosubscribeToBlockHeadersWithChainLocksWithRequest:(BlockHeadersWithChainLocksRequest *)request eventHandler:(void(^)(BOOL done, BlockHeadersWithChainLocksResponse *_Nullable response, NSError *_Nullable error))eventHandler;


#pragma mark subscribeToTransactionsWithProofs(TransactionsWithProofsRequest) returns (stream TransactionsWithProofsResponse)

- (void)subscribeToTransactionsWithProofsWithRequest:(TransactionsWithProofsRequest *)request eventHandler:(void(^)(BOOL done, TransactionsWithProofsResponse *_Nullable response, NSError *_Nullable error))eventHandler;

- (GRPCProtoCall *)RPCTosubscribeToTransactionsWithProofsWithRequest:(TransactionsWithProofsRequest *)request eventHandler:(void(^)(BOOL done, TransactionsWithProofsResponse *_Nullable response, NSError *_Nullable error))eventHandler;


@end

/**
 * Basic service implementation, over gRPC, that only does
 * marshalling and parsing.
 */
@interface Core : GRPCProtoService<Core>
- (instancetype)initWithHost:(NSString *)host NS_DESIGNATED_INITIALIZER;
+ (instancetype)serviceWithHost:(NSString *)host;
@end

NS_ASSUME_NONNULL_END

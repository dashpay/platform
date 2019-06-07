#import "TransactionsFilterStream.pbobjc.h"

#import <ProtoRPC/ProtoService.h>
#import <ProtoRPC/ProtoRPC.h>
#import <RxLibrary/GRXWriteable.h>
#import <RxLibrary/GRXWriter.h>



NS_ASSUME_NONNULL_BEGIN

@protocol TransactionsFilterStream <NSObject>

#pragma mark subscribeToTransactionsWithProofs(TransactionsWithProofsRequest) returns (stream TransactionsWithProofsResponse)

- (void)subscribeToTransactionsWithProofsWithRequest:(TransactionsWithProofsRequest *)request eventHandler:(void(^)(BOOL done, TransactionsWithProofsResponse *_Nullable response, NSError *_Nullable error))eventHandler;

- (GRPCProtoCall *)RPCTosubscribeToTransactionsWithProofsWithRequest:(TransactionsWithProofsRequest *)request eventHandler:(void(^)(BOOL done, TransactionsWithProofsResponse *_Nullable response, NSError *_Nullable error))eventHandler;


@end

/**
 * Basic service implementation, over gRPC, that only does
 * marshalling and parsing.
 */
@interface TransactionsFilterStream : GRPCProtoService<TransactionsFilterStream>
- (instancetype)initWithHost:(NSString *)host NS_DESIGNATED_INITIALIZER;
+ (instancetype)serviceWithHost:(NSString *)host;
@end

NS_ASSUME_NONNULL_END

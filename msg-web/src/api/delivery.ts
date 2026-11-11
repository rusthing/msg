/**
 * # 投递记录的业务动作
 *
 * 投递记录本身由消息处理链路生成，CRUD 客户端（`crudOf`）只够读和改状态。
 * 「再次投递」不在 CRUD 的形状里——它是 `POST /msg/delivery/redeliver`，
 * 请求体只有投递 ID，其余全部由服务端按原记录取——所以在 `api/crud.ts`
 * 之外单开一个文件，别让通用客户端背上特例。
 *
 * ## 服务端做的事
 *
 * 按这条投递**当时**拆出来的「目标 × 渠道」重发一遍：三级状态回到「投递中」，
 * 重新发布投递任务，结果仍由渠道消费者异步回写。不新建投递记录
 * （`business_id + business_trigger_ms` 是幂等键，另造一条等于伪造业务时刻）。
 */

import { apiPost } from './client';
import type { Id } from './types';

/** # 再次投递的结果 */
export interface RedeliverResult {
  /** 本次拆出的投递任务数（目标 × 渠道） */
  taskCount: number;
  /** 成功发布到渠道投递队列的任务数 */
  publishedCount: number;
  /** 发布失败的任务数（已按失败回写渠道状态） */
  failedCount: number;
}

export const deliveryApi = {
  /**
   * 再次投递
   *
   * `id` 是投递记录主键（`u64` 字符串，不要 `Number()`）。
   *
   * 返回的计数**只到「任务已发布」**为止，不代表渠道已经送达——真正的送达结果
   * 要等消费者回写，刷新后看「投递渠道任务」与「渠道投递日志」。
   */
  redeliver: (id: Id): Promise<RedeliverResult> =>
    apiPost<RedeliverResult>('/msg/delivery/redeliver', { id }),
};

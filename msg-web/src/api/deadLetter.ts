/**
 * # 死信的业务动作
 *
 * 死信只读，唯一的写动作是「标记已处理」：`POST /msg/dead-letter/handle`，
 * 请求体只有死信 ID。与投递记录的「再次投递」一样，它不在 CRUD 的形状里，
 * 所以单开一个文件。
 *
 * ## 服务端只改一件事
 *
 * `handled` 置为 `true`。原始负载、转入原因、投递次数都是落库时的证据，
 * 不随这个动作变化；重复点击结果相同。
 */

import { apiPost } from './client';
import type { Id, Row } from './types';

export const deadLetterApi = {
  /**
   * 标记死信已处理
   *
   * `id` 是死信主键（`u64` 字符串，不要 `Number()`）。
   */
  markHandled: (id: Id): Promise<Row> => apiPost<Row>('/msg/dead-letter/handle', { id }),
};

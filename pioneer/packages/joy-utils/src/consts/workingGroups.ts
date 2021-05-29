import { WorkingGroupKey } from '@joystream/types/common';
export const apiModuleByGroup: { [k in WorkingGroupKey]: string } = {
  Storage: 'storageWorkingGroup',
  Content: 'contentDirectoryWorkingGroup',
  Operations: 'operationsWorkingGroup',
  Gateway: 'gatewayWorkingGroup'
};

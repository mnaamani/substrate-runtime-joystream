import tap from 'tap';
import { initConfig } from '../../utils/config';
import { registerJoystreamTypes } from '@nicaea/types';
import { closeApi } from '../impl/closeApi';
import { ApiWrapper } from '../../utils/apiWrapper';
import { WsProvider, Keyring } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';
import { setTestTimeout } from '../../utils/setTestTimeout';
import { membershipTest, createKeyPairs } from '../impl/membershipCreation';
import {
  addWorkerOpening,
  applyForOpening,
  acceptApplications,
  terminateApplications,
  unsetLead,
  addLeaderOpening,
  beginLeaderApplicationReview,
  fillLeaderOpening,
  leaveRole,
} from './impl/workingGroupModule';
import BN from 'bn.js';

tap.mocha.describe('Worker application happy case scenario', async () => {
  initConfig();
  registerJoystreamTypes();

  const nKeyPairs: KeyringPair[] = new Array();
  const leadKeyPair: KeyringPair[] = new Array();

  const keyring = new Keyring({ type: 'sr25519' });
  const N: number = +process.env.WORKING_GROUP_N!;
  const paidTerms: number = +process.env.MEMBERSHIP_PAID_TERMS!;
  const nodeUrl: string = process.env.NODE_URL!;
  const sudoUri: string = process.env.SUDO_ACCOUNT_URI!;
  const applicationStake: BN = new BN(process.env.WORKING_GROUP_APPLICATION_STAKE!);
  const roleStake: BN = new BN(process.env.WORKING_GROUP_ROLE_STAKE!);
  const durationInBlocks: number = 28;
  const openingActivationDelay: BN = new BN(100);

  const provider = new WsProvider(nodeUrl);
  const apiWrapper: ApiWrapper = await ApiWrapper.create(provider);
  const sudo: KeyringPair = keyring.addFromUri(sudoUri);
  const nonMemberKeyPairs = createKeyPairs(keyring, N);

  setTestTimeout(apiWrapper, durationInBlocks);
  membershipTest(apiWrapper, nKeyPairs, keyring, N, paidTerms, sudoUri);
  membershipTest(apiWrapper, leadKeyPair, keyring, N, paidTerms, sudoUri);

  let leadOpenignId: BN;
  tap.test(
    'Add lead opening',
    async () =>
      (leadOpenignId = await addLeaderOpening(
        apiWrapper,
        nKeyPairs,
        sudo,
        applicationStake,
        roleStake,
        openingActivationDelay
      ))
  );
  tap.test(
    'Apply for lead opening',
    async () => await applyForOpening(apiWrapper, leadKeyPair, sudo, applicationStake, roleStake, leadOpenignId, false)
  );
  tap.test('Begin lead application review', async () => beginLeaderApplicationReview(apiWrapper, sudo, leadOpenignId));
  tap.test('Fill lead opening', async () => fillLeaderOpening(apiWrapper, leadKeyPair, sudo, leadOpenignId));

  let openignId: BN;
  tap.test(
    'Add worker opening',
    async () =>
      (openignId = await addWorkerOpening(
        apiWrapper,
        nKeyPairs,
        leadKeyPair[0],
        sudo,
        applicationStake,
        roleStake,
        openingActivationDelay
      ))
  );
  tap.test('Apply for worker opening, expect failure', async () =>
    applyForOpening(apiWrapper, nKeyPairs, sudo, applicationStake, roleStake, openignId, true)
  );
  tap.test('Begin accepting worker applications', async () =>
    acceptApplications(apiWrapper, leadKeyPair[0], sudo, openignId)
  );
  tap.test('Apply for worker opening as non-member, expect failure', async () =>
    applyForOpening(apiWrapper, nonMemberKeyPairs, sudo, applicationStake, roleStake, openignId, true)
  );
  tap.test('Apply for worker opening as member', async () =>
    applyForOpening(apiWrapper, nKeyPairs, sudo, applicationStake, roleStake, openignId, false)
  );
  tap.test('Terminate worker applicaitons', async () =>
    terminateApplications(apiWrapper, nKeyPairs, leadKeyPair[0], sudo)
  );
  tap.test('Unset lead', async () => unsetLead(apiWrapper, sudo));

  tap.test('Leaving lead role', async () => leaveRole(apiWrapper, leadKeyPair, sudo));

  closeApi(apiWrapper);
});
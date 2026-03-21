import { createRouter, createWebHashHistory } from 'vue-router';
import VideoDownloadPage from '@/modules/download/pages/VideoDownloadPage.vue';
import LiveRecordingPage from '@/modules/recording/pages/LiveRecordingPage.vue';
import RecordingAccountCreatePage from '@/modules/recording/pages/RecordingAccountCreatePage.vue';
import RecordingAccountLogsPage from '@/modules/recording/pages/RecordingAccountLogsPage.vue';
import HistoryPage from '@/modules/tasks/pages/HistoryPage.vue';
import BatchDetailPage from '@/modules/tasks/pages/BatchDetailPage.vue';
import TaskDetailPage from '@/modules/tasks/pages/TaskDetailPage.vue';
import SettingsPage from '@/modules/settings/pages/SettingsPage.vue';

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/download/video' },
    {
      path: '/download/video',
      name: 'video-download',
      component: VideoDownloadPage,
      meta: {
        primaryNav: 'download',
        titleKey: 'routes.videoDownload'
      }
    },
    {
      path: '/record/live',
      name: 'live-recording',
      component: LiveRecordingPage,
      meta: {
        primaryNav: 'record',
        titleKey: 'routes.liveRecording'
      }
    },
    {
      path: '/record/account/create',
      name: 'recording-account-create',
      component: RecordingAccountCreatePage,
      meta: {
        primaryNav: 'record',
        titleKey: 'routes.recordingAccountCreate',
        standalone: true
      }
    },
    {
      path: '/record/account/:accountId/edit',
      name: 'recording-account-edit',
      component: RecordingAccountCreatePage,
      meta: {
        primaryNav: 'record',
        titleKey: 'routes.recordingAccountEdit',
        standalone: true
      }
    },
    {
      path: '/record/account/:accountId/logs',
      name: 'recording-account-logs',
      component: RecordingAccountLogsPage,
      meta: {
        primaryNav: 'record',
        titleKey: 'routes.recordingAccountLogs',
        standalone: true
      }
    },
    {
      path: '/tasks/history',
      name: 'task-history',
      component: HistoryPage,
      meta: {
        primaryNav: 'tasks',
        titleKey: 'routes.taskHistory'
      }
    },
    {
      path: '/tasks/batch/:batchId',
      name: 'batch-detail',
      component: BatchDetailPage,
      meta: {
        primaryNav: 'tasks',
        titleKey: 'routes.batchDetail',
        standalone: true
      }
    },
    {
      path: '/tasks/detail/:taskId',
      name: 'task-detail',
      component: TaskDetailPage,
      meta: {
        primaryNav: 'tasks',
        titleKey: 'routes.taskDetail',
        standalone: true
      }
    },
    {
      path: '/settings',
      name: 'settings',
      component: SettingsPage,
      meta: {
        primaryNav: 'settings',
        titleKey: 'routes.settings'
      }
    }
  ]
});

export default router;

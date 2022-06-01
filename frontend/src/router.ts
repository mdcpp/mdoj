import { createRouter, createWebHistory, RouteRecordRaw } from "vue-router"
import Home from '@/views/Home.vue';
import Problems from '@/views/Problems.vue';
import Contests from '@/views/Contests.vue';
import Status from '@/views/Status.vue';
import Rank from '@/views/Rank.vue';
import About from '@/views/About.vue';

const routes: RouteRecordRaw[] = [
    {
        path: "/",
        component: Home
    },
    {
        path: "/problems",
        component: Problems
    },
    {
        path: "/contests",
        component: Contests
    },
    {
        path: "/status",
        component: Status
    },
    {
        path: "/rank",
        component: Rank
    },
    {
        path: "/about",
        component: About
    },

]

const router = createRouter({
    history: createWebHistory(),
    routes
})

export default router
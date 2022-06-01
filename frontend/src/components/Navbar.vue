<template>
    <nav>
        <div>
            <div>
                <Link to="/">
                    <h1>MDOJ</h1>
                </Link>
            </div>
            <div>
                <ul>
                    <li>
                        <Link to="/problems">
                        <code-icon />
                        Problems
                        </Link>
                    </li>

                    <li>
                        <Link to="/contests">
                        <flag-icon />
                        Contests
                        </Link>
                    </li>

                    <li>
                        <Link to="/status">
                        <terminal-icon />
                        Status
                        </Link>
                    </li>

                    <li>
                        <Link to="/rank">
                        <trending-up-icon />
                        Rank
                        </Link>
                    </li>

                    <li>
                        <Link to="/about">
                        <information-circle-icon />
                        About
                        </Link>
                    </li>
                    <li>
                        <Button>
                            <login-icon />
                            Login
                        </Button>
                    </li>
                </ul>
            </div>
        </div>
    </nav>
</template>

<style lang="sass" scoped>
nav
    @apply bg-gray-800 py-4 px-2 rounded-b sticky select-none top-0

    & > div
        @apply container flex flex-wrap justify-between items-center mx-auto px-3

ul
    @apply flex flex-row w-full justify-between items-center

li 
    @apply pl-5

h1  
    @apply text-2xl text-white

</style>

<script lang="ts" setup>
import { CodeIcon, InformationCircleIcon, LoginIcon, FlagIcon, TrendingUpIcon, TerminalIcon } from '@heroicons/vue/outline';
import Button from '@/components/Button.vue';
import Link from '@/components/Link.vue';
</script>
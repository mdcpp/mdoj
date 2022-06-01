<template>
    <router-link :to="to">
        <slot></slot>
    </router-link>
</template>

<style lang="sass">
a
    @apply transition-colors flex hover:text-white

svg
    @apply h-5 self-center mr-1

</style>

<script lang="ts" setup>

defineProps<{ to: String }>();
</script>
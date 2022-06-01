<template>

  <Navbar />
  <div>
    <router-view></router-view>
  </div>
  
</template>

<style lang="sass" scoped>
</style>

<script lang="ts" setup>
import Navbar from '@/components/Navbar.vue';
</script>
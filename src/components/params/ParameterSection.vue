<script setup lang="ts">
import { Settings2 } from "@lucide/vue";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import TechnicalParams from "@/components/params/TechnicalParams.vue";
import EconomicParams from "@/components/params/EconomicParams.vue";
import CapacityAlgorithmParams from "@/components/params/CapacityAlgorithmParams.vue";
import StoragePeriods from "@/components/params/StoragePeriods.vue";
import SchemeGoalParams from "@/components/params/SchemeGoalParams.vue";

const defaultTab = "technical";
</script>

<template>
  <Card>
    <CardHeader>
      <CardTitle class="flex items-center gap-2">
        <Settings2 class="size-5 text-emerald-600" />
        参数输入与确认
      </CardTitle>
      <CardDescription>
        上传文件后自动读取参数；界面修改将覆盖上传值；“储能时段 / 输配电费方案 / 优化目标”仅在界面操作
      </CardDescription>
    </CardHeader>
    <CardContent>
      <Tabs :default-value="defaultTab" class="w-full">
        <TabsList class="flex-wrap justify-start">
          <TabsTrigger value="technical">技术参数</TabsTrigger>
          <TabsTrigger value="economic">经济评价参数</TabsTrigger>
          <TabsTrigger value="capacity">择优范围与算法</TabsTrigger>
          <TabsTrigger value="periods">储能时段</TabsTrigger>
          <TabsTrigger value="scheme">方案与目标</TabsTrigger>
        </TabsList>

        <TabsContent value="technical" class="pt-4">
          <TechnicalParams />
        </TabsContent>
        <TabsContent value="economic" class="pt-4">
          <EconomicParams />
        </TabsContent>
        <TabsContent value="capacity" class="pt-4">
          <CapacityAlgorithmParams />
        </TabsContent>
        <TabsContent value="periods" class="pt-4">
          <StoragePeriods />
        </TabsContent>
        <TabsContent value="scheme" class="pt-4">
          <SchemeGoalParams />
        </TabsContent>
      </Tabs>
    </CardContent>
  </Card>
</template>

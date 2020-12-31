Vue.component('notify-config', {
    props: {
        title: String,
        config: Object
    },

    data() {
        return {
            sound_options: [
                { label: "Bounty rune",         value: "bounty_rune.mp3" },
                { label: "Power rune",          value: "power_rune.mp3" },
                { label: "Neutral items",       value: "neutral_items.mp3" },
                { label: "Observer ward",       value: "observer_ward.mp3" },
                { label: "Tomb of knowledge",   value: "tomb_of_knowledge.mp3" },
                { label: "Buyback ready",       value: "buyback_ready.mp3" }
            ]
        }
    },

    methods: {
        async trigger(event) {
            await axios.post('/api/trigger', this.config.notify.action)
        }
    },

    // computed: {
    //     notify_before_sec: {
    //         get() {
    //             return this.rune.spawn_interval_sec - this.rune.notify_sec;
    //         },
    //         set(value) {
    //             this.rune.notify_sec = this.rune.spawn_interval_sec - value;
    //         }
    //     }
    // },

    template: `
        <el-card class="box-card">
            <div v-if="title" slot="header" class="clearfix">
                <span>{{title}}</span>
            </div>
            <el-form v-if="config.notify" label-position="right" label-width="110px" :model="config">
                <el-form-item label="Enabled">
                    <el-switch v-model="config.notify.enabled"/>
                </el-form-item>
                <el-row type="flex">
                    <el-col :span="6">                  
                        <el-form-item label="Notify before">
                            <el-input-number v-model="config.notify.before_sec" :min="0" :max="60"/> [s]
                        </el-form-item>
                        
                        <template v-if="config.notify.action">
                            <el-form-item label="Notify Action">
                                <el-radio-group v-model="config.notify.action.type">
                                    <el-radio label="beep" >Beep</el-radio>
                                    <el-radio label="sound" >Sound</el-radio>
                                    <el-radio label="playfile" >Play file</el-radio>
                                </el-radio-group>
                            </el-form-item>
                            
                            <template v-if="config.notify.action.type == 'beep'">
                                <el-form-item label="Frequency">
                                    <el-input-number v-model="config.notify.action.freq" :min="200" :max="1000" :step="50"/> [Hz]
                                </el-form-item>
                                <el-form-item label="Duration">
                                    <el-input-number v-model="config.notify.action.duration_ms" :min="50" :max="2000" step="50"/> [ms]
                                    <el-button @click="trigger" icon="el-icon-video-play" :disabled="config.notify.action.freq == null || config.notify.action.duration_ms == null"/>
                                </el-form-item>
                            </template>
                            
                            <template v-if="config.notify.action.type == 'sound'">
                                <el-form-item label="Sound">
                                    <el-select v-model="config.notify.action.sound" placeholder="Select">
                                        <el-option
                                          v-for="sound in sound_options"
                                          :key="sound.value"
                                          :label="sound.label"
                                          :value="sound.value">
                                        </el-option>
                                    </el-select>
                                    <el-button @click="trigger" icon="el-icon-video-play" :disabled="config.notify.action.sound == null"/>
                                </el-form-item>
                            </template>
                            
                            <template v-if="config.notify.action.type == 'playfile'">
                                <el-form-item label="File">
                                    <el-input v-model="config.notify.action.path"></el-input>
                                    <el-button @click="trigger" icon="el-icon-video-play" :disabled="config.notify.action.path == null"/>
                                </el-form-item>
                            </template>
                        </template>

                        <el-form-item>
                            <el-button type="primary" @click="$emit('do-save')">Save</el-button>
                        </el-form-item>
                    </el-col>
                    <el-col :span="6">
                        <template v-if="config.spawn">
                            <el-form-item label="Spawn interval">
                                <el-input-number v-model="config.spawn.interval_sec" :min="1" :max="600" /> [s]
                            </el-form-item>
                            <el-form-item label="Spawn first">
                                <el-input-number v-model="config.spawn.first_sec" :min="0" :max="600" /> [s]
                            </el-form-item>
                        </template>                   
                    </el-col>
                </el-row>
            </el-form>
        </el-card>
    `
})

Vue.component('install', {

    data() {
        return {
            install: {}
        }
    },

    methods: {
        async doInstall(event) {
            try {
                let res = await axios.post('/api/install')
                if (res.status === 200) {
                    this.$message({
                        showClose: true,
                        message: 'Installation successful',
                        type: 'success'
                    });
                }
                await this.doLoad();
            } catch (e) {
                this.$message({
                    showClose: true,
                    message: 'Unable to install game state integration file',
                    type: 'error'
                });
                console.error("Failed to install game state integration file", e)
            }
        },
        async doLoad() {
            const loading = this.$loading({
                lock: true,
                text: 'Loading . . .',
                background: 'rgba(255, 255, 255, 0.6)'
            });
            try {
                this.install = (await axios.get('/api/install')).data;
            } catch (e) {
                this.$message({
                    showClose: true,
                    message: 'Unable to detect install state',
                    type: 'error'
                });
                console.error("Failed to detect install state", e)
            }
            this.visible = true
            loading.close()
        }
    },

    async created() {
        await this.doLoad()
    },

    template: `
        <el-card class="box-card" style="width:600px">
            <div slot="header" class="clearfix">
                <span>Game state integration</span>
            </div>
            <div v-if="install.dota_announcer_integration_file_exists">
                <el-button icon="el-icon-document-checked" type="success" round>Installed</el-button>
                <p>Game state integration file is installed correctly</p>
            </div>
            <div v-else-if="!install.dota_dir">
                <p>Sorry but there is no Dota2 detected in the system</p>
            </div>
            <div v-else>
                <el-collapse value="1">
                    <el-collapse-item title="AutoMagic Installation" name="1">
                        <div>
                            Will create <span style="font-size: smaller">{{install.announcer_integration_file_name}}</span> file in<br/><span style="font-size: smaller">{{install.dota_gamestate_integration_dir}}</span>
                            <p>You can also use manual installation if you want to inspect the file first</p>
                        </div>
                        <el-button icon="el-icon-document-add" type="warning" plain @click="doInstall">Install</el-button>
                    </el-collapse-item>
                    <el-collapse-item title="Manual Installation" name="2">
                        <ol>
                            <li>Download file <el-link type="primary" icon="el-icon-download" :href="install.announcer_integration_file_name">{{install.announcer_integration_file_name}}</el-link></li>
                            <li>Copy it to the folder<br/><span style="font-size: smaller">{{install.dota_gamestate_integration_dir}}</span></li>
                            <li>When done refresh this page</li>
                        </ol>
                    </el-collapse-item>
                </el-collapse>
            </div>
        </el-card>
    `
})
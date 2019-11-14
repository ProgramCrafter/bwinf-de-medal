/*
 * Cross-domain task proxy implementation for Bebras task API - v1.0 - 08/2014
 *
 * This file implements a "TaskProxyManager" object in the global scope so that
 * platforms using this file can just call
 * "myTask = TaskProxyManager.getTaskProxy(task_id)" to get the task object
 * associated to a task loaded in an iframe. Then they can just call
 * "myTask.myFunction()", with function of the API, the task object will take
 * care of message sending and receiving with the actual task.
 * 'task_id' here refers to the "id" attribute of the iframe in which a task is
 * loaded.
 *
 * It depends on jQuery (though this could be removed) and jschannel.
 *
 */


/**
 * Global objects
 */

var TaskProxyManager = {
   tasks: {},
   platforms: {},
   getRandomID: function() {
      var low = Math.floor(Math.random() * 922337203).toString();
      var high = Math.floor(Math.random() * 2000000000).toString();
      return high + low;
   },
   getTaskProxy: function(idFrame, success, force, error) {
      var errorFun = error ? error : function() {console.error(arguments);}
      if (TaskProxyManager.tasks[idFrame] && !force) {
         success(TaskProxyManager.tasks[idFrame]);
      } else {
         if (force) {
            TaskProxyManager.deleteTaskProxy(idFrame);
         }
         $('#'+idFrame).each(function() {
            TaskProxyManager.tasks[idFrame] = new Task($(this), function() {
               if (idFrame in TaskProxyManager.platforms) {
                  TaskProxyManager.tasks[idFrame].setPlatform(TaskProxyManager.platforms[idFrame]);
               }
               success(TaskProxyManager.tasks[idFrame]);
            }, errorFun);
         });
      }
   },
   setPlatform: function(task, platform) {
      TaskProxyManager.platforms[task.Id] = platform;
      TaskProxyManager.tasks[task.Id].setPlatform(platform);
   },
   deleteTaskProxy: function(idFrame) {
      var task = TaskProxyManager.tasks[idFrame];
      if (task && task.chan) {
         task.chan.destroy();
      }
      delete(TaskProxyManager.tasks[idFrame]);
      delete(TaskProxyManager.platforms[idFrame]);
   },
   getUrl: function(taskUrl, sToken, sPlatform, prefix) {
      var channelId = (prefix ? prefix : '')+this.getRandomID();
      if (taskUrl.indexOf('?') == -1) {
         // the idea is not to change the base url even if we change token, so we put token after #
         taskUrl = taskUrl + '?';
      } else {
         taskUrl = taskUrl + '&';
      }
      return taskUrl+'sToken='+encodeURIComponent(sToken)+'&sPlatform='+encodeURIComponent(sPlatform)+'&channelId='+encodeURIComponent(channelId);
   }
};

TaskProxyManager.getGraderProxy = TaskProxyManager.getTaskProxy;

/**
 * Enable / Disable messages debug
 */
var platformDebug = false;

/*
 * Task object, created from an iframe DOM element
 */
function Task(iframe, success, error) {
   this.iframe = iframe;
   this.Id = iframe.attr('id');
   this.platformSet = false;
   function getUrlParameterByName(name, url) {
       var regex = new RegExp("[\\?&]" + name + "=([^&#]*)"),
           results = regex.exec(url);
       return results === null ? "" : decodeURIComponent(results[1].replace(/\+/g, " "));
   }
   var nbSecs = 0;
   var checkInterval = setInterval(function() {
      if (nbSecs > 15) {
         error();
         clearInterval(checkInterval);
         checkInterval = null;
      }
      nbSecs = nbSecs + 1;
   }, 1000);
   this.chan = Channel.build({
      window: iframe[0].contentWindow,
      origin: "*",
      scope: getUrlParameterByName('channelId', iframe[0].src),
      onReady: function() {
         if (checkInterval) {
            clearInterval(checkInterval);
            success();
         }
      }
   });
   this.setPlatform = function(platform) {
      this.platform = platform;
      var self = this;
      if (this.platformSet) {
         // in this case, the bound functions will call the new platform
         return;
      }
      this.chan.bind('platform.validate', function (trans, mode) {
         self.platform.validate(mode, trans.complete, trans.error);
         trans.delayReturn(true);
         console.error('platform.validate with mode '+mode);
      });
      this.chan.bind('platform.getTaskParams', function (trans, keyDefault) {
         var key = keyDefault ? keyDefault[0] : undefined;
         var defaultValue = keyDefault ? keyDefault[1] : undefined;
         self.platform.getTaskParams(key, defaultValue, trans.complete, trans.error);
         trans.delayReturn(true);
      });
      this.chan.bind('platform.showView', function (trans, view) {
         self.platform.showView(view, trans.complete, trans.error);
         trans.delayReturn(true);
      });
      this.chan.bind('platform.askHint', function (trans, hintToken) {
         self.platform.askHint(hintToken, trans.complete, trans.error);
         trans.delayReturn(true);
      });
      this.chan.bind('platform.updateHeight', function (trans, height) {
         self.platform.updateHeight(height, trans.complete, trans.error);
         trans.delayReturn(true);
      });
      this.chan.bind('platform.openUrl', function (trans, url) {
         self.platform.openUrl(url, trans.complete, trans.error);
         trans.delayReturn(true);
      });
      self.platformSet = true;
   };
}

Task.prototype.getSourceId = function() {
   return this.Id;
};

Task.prototype.getTargetUrl = function() {
   return this.iframe.attr('src');
};

Task.prototype.getTarget = function() {
   return this.iframe[0].contentWindow;
};

Task.prototype.getDomain = function() {
   var url = this.getTargetUrl();
   return url.substr(0, url.indexOf('/', 7));
};

/**
 * Task API functions
 */
Task.prototype.load = function(views, success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.load",
      params: views,
      timeout: 10000,
      success: success,
      error: error
   });
};

Task.prototype.unload = function(success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.unload",
      timeout: 1000,
      error: error,
      success: success
   });
};

Task.prototype.getHeight = function(success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.getHeight",
      timeout: 100,
      error: error,
      success: success
   });
};

Task.prototype.updateToken = function(token, success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.updateToken",
      params: token,
      timeout: 10000,
      error: error,
      success: success
   });
};

Task.prototype.getMetaData = function(success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.getMetaData",
      timeout: 500,
      error: error,
      success: success
   });
};

Task.prototype.getAnswer = function(success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.getAnswer",
      error: error,
      success: success,
      timeout: 1000
   });
};

Task.prototype.reloadAnswer = function(answer, success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.reloadAnswer",
      params: answer,
      error: error,
      success: success,
      timeout: 1000
   });
};

Task.prototype.getState = function(success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.getState",
      error: error,
      success: success,
      timeout: 1000
   });
};

Task.prototype.reloadState = function(state, success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.reloadState",
      params: state,
      error: error,
      success: success,
      timeout: 1000
   });
};

Task.prototype.getViews = function(success, error) {
   if (!error) error = function() {console.error(arguments)};
   this.chan.call({method: "task.getViews",
      error: error,
      success: success,
      timeout: 1000
   });
};

Task.prototype.showViews = function(views, success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.showViews",
      params: views,
      error: error,
      success: success,
      timeout: 1000
   });
};

Task.prototype.gradeAnswer = function(answer, answerToken, success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   var newSuccess = function(successMonoArg) {
      if (successMonoArg instanceof Array) {
         var argzero = successMonoArg[0];
         var argone  = (successMonoArg.length > 1) ? successMonoArg[1] : null;
         var argtwo  = (successMonoArg.length > 2) ? successMonoArg[2] : null;
         var argthree  = (successMonoArg.length > 3) ? successMonoArg[3] : null;
         success(argzero, argone, argtwo, argthree);
      } else {
         success(successMonoArg);
      }
   };
   this.chan.call({method: "task.gradeAnswer",
      params: [answer, answerToken],
      error: error,
      success: newSuccess,
      timeout: 30000
   });
};

Task.prototype.getResources = function(success, error) {
   if (!error) error = function(errMsg) {console.error(errMsg)};
   this.chan.call({method: "task.getResources",
      params: [],
      error: error,
      success: success,
      timeout: 2000
   });
};

// for grader.gradeTask
Task.prototype.gradeTask = Task.prototype.gradeAnswer;

/*
 * Platform object definition, created from a Task object (see below)
 */

function Platform(task) {
   this.task = task;
}

Platform.prototype.getTask = function() {
   return this.task;
};

/*
 * Simple prototypes for platform API functions, to be overriden by your
 * platform's specific functions (for each platform object)
 */

Platform.prototype.validate = function(mode, success, error) {error('platform.validate is not defined');};
Platform.prototype.showView = function(views, success, error) {error('platform.validate is not defined');};
Platform.prototype.askHint = function(platformToken, success, error) {error('platform.validate is not defined');};
Platform.prototype.updateHeight = function(height, success, error) {this.task.iframe.height(parseInt(height)+40);success();};
Platform.prototype.openUrl = function(url, success, error) {error('platform.openUrl is not defined!');};
Platform.prototype.getTaskParams = function(key, defaultValue, success, error) {
   var res = {minScore: -3, maxScore: 10, randomSeed: 0, noScore: 0, readOnly: false, options: {}};
   if (key) {
      if (key !== 'options' && key in res) {
         res = res[key];
      } else if (res.options && key in res.options) {
         res = res.options[key];
      } else {
         res = (typeof defaultValue !== 'undefined') ? defaultValue : null;
      }
   }
   success(res);
};

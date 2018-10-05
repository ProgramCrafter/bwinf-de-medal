"use strict";

function hash_to_dict() {
  var splithash = location.hash.substr(1).split('&').filter(function(x){return x.length>0}).map(function(x){return x.split('=')});
  var dict = {};
  for (var i in splithash) {
    for (var j in splithash[i]) {
      dict[splithash[i][0]] = splithash[i][j];
    }
  }
  return dict;
}

window.hashdict = hash_to_dict();


window.load_task_object = function (callback) {
  console.log(callback);
  $.get("/load/" + window.hashdict["taskid"], {},
        function(data) {
          callback(data);
        }, "json").fail(function(){
          alert("Load failed.");
        })
}

window.save_task_object = function (object, grade, callback) {
  if (!grade) grade = 0;
  if (!callback) callback = function(data){};

  var params = {
    csrf: window.hashdict["csrftoken"],
    data: JSON.stringify(object),
    grade: JSON.stringify(grade)
  }
  $.post("/save/" + window.hashdict["taskid"], params, callback, "json").fail(function(){
    alert("Save failed.");
  });
}


window.load_subtask_object = function (subtaskname, callback) {
  var params = {
    subtask: subtaskname
  }
  $.get("/load/" + window.hashdict["taskid"], params,
        function(data) {
          callback(data);
        }, "json").fail(function(){
          alert("Load failed.");
        })
}

window.save_subtask_object = function (subtaskname, object, grade, callback) {
  if (!grade) grade = 0;
  if (!callback) callback = function(data){};

  var params = {
    subtask: subtaskname,
    csrf: window.hashdict["csrftoken"],
    data: JSON.stringify(object),
    grade: JSON.stringify(grade)
  }
  $.post("/save/" + window.hashdict["taskid"], params, callback, "json").fail(function(){
    alert("Save failed.");
  });
}

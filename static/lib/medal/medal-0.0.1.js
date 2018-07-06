
function hash_to_dict() {
  splithash = location.hash.substr(1).split('&').filter(function(x){return x.length>0}).map(function(x){return x.split('=')});
  dict = {};
  for (i in splithash) {
    for (j in splithash[i]) {
      dict[splithash[i][0]] = splithash[i][j];
    }
  }
  return dict;
}

window.hashdict = hash_to_dict();


window.load_task_object = function (callback) {
  params = {
    task: window.hashdict["taskid"]
  }
  $.get("/load", params,
        function(data) {
          callback(data);
        }, "json").fail(function(){
          alert("Load failed.");
        })
}

window.save_task_object = function (object, callback) {
  if (!callback) callback = function(data){}; // is this necessary?

  params = {
    task: window.hashdict["taskid"],
    csrf: window.hashdict["csrftoken"],
    value: JSON.stringify(object)
  }
  $.post("/save", params, callback, "json").fail(function(){
    alert("Save failed.");
  });
}


window.load_subtask_object = function (subtaskname, callback) {
  params = {
    task: window.hashdict["taskid"],
    subtask: subtaskname
  }
  $.get("/load", params,
        function(data) {
          callback(data);
        }, "json").fail(function(){
          alert("Load failed.");
        })
}

window.save_subtask_object = function (subtaskname, object, callback) {
  if (!callback) callback = function(data){}; // is this necessary?

  params = {
    task: window.hashdict["taskid"],
    subtask: subtaskname,
    csrf: window.hashdict["csrftoken"],
    value: JSON.stringify(object)
  }
  $.post("/save", params, callback, "json").fail(function(){
    alert("Save failed.");
  });
}
